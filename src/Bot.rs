use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use anyhow::{Error, Result};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;
use futures_util::{SinkExt, StreamExt, TryStreamExt};
use futures_util::stream::Next;
use tokio::sync::mpsc::{Receiver, Sender, UnboundedSender};

type Bots = HashMap<String, Arc<RefCell<Bot>>>;
type Handle = fn(CQMessage, &mut Bot) -> Result<()>;
type HandleVec = Vec<Handle>;
type WsStream = Arc<RefCell<WebSocketStream<TcpStream>>>;

unsafe impl Send for Bot {}

unsafe impl Sync for Bot {}

/// 使用ws-server模式,也就是cqhttp主动连接至本框架
/// 此模式可以同时被多个bot连接
/// ```
/// let builder = BotBuilder::new();
/// builder.add_handle(handle1); //handle是一个function
/// builder.add_handle(handle2);
/// builder.start("0.0.0.0:1611").await;
/// ```
/// 实际内部维护一个Bot组
pub struct BotBuilder {
    bots: Bots,
    handle_vec: Option<HandleVec>,
}
/// 使用ws-client连接
/// todo
pub struct Bot {
    msg_sender: Option<Sender<Message>>,
    msg_receiver: Option<Receiver<RecMessage>>,
    handle_vec: Option<HandleVec>,
    addr: Option<SocketAddr>,
}

#[derive(Clone, Copy)]
pub struct CQMessage {}

pub struct RecMessage {
    addr: String,
    msg: Message
}

impl BotBuilder {
    fn insent_bot(&mut self, key: String, value: Arc<RefCell<Bot>>) {
        self.bots.insert(key, value);
    }

    fn remove_bot(&mut self, key: &str) {
        self.bots.remove(key);
    }

    fn get_bot(&self, key: &str) -> Option<&Arc<RefCell<Bot>>>{
        self.bots.get(key)
    }

    fn get_handle(&mut self) -> Option<HandleVec> {
        self.handle_vec.take()
    }
}
unsafe impl Send for BotBuilder{}
unsafe impl Sync for BotBuilder{}

impl BotBuilder {
    pub fn new() -> Self {
        BotBuilder {
            bots: HashMap::new(),
            handle_vec: Some(Vec::new()),
        }
    }

    /// 可以最多添加 int_max个function
    /// 好像是废话
    pub fn add_handle(&mut self, handle_fn: Handle) {
        let vec = self.handle_vec.take();
        if let Some(mut vec) = vec {
            vec.push(handle_fn);

            self.handle_vec = Some(vec);
        }
    }

    /// 启动
    /// 如果想启动任务请使用
    /// ```
    /// tokio::spawn(BotBuilder::start(builder));
    /// ```
    pub async fn start(mut builder: BotBuilder, addr: &str) {
        let socket;
        socket = TcpListener::bind(&addr).await.expect("bind error");

        let (bot_in, mut bot_out) = tokio::sync::mpsc::channel::<Bot>(5);

        tokio::spawn(async move {
            let bot = bot_out.recv().await;
            if bot.is_none() {
                // error
            }
            let mut bot = bot.unwrap();
            let addr = bot.addr.take();
            let receive = bot.msg_receiver.take();
            if addr.is_none() || receive.is_none() {
                //error
            }

            let addr = addr.unwrap();
            let mut receive = receive.unwrap();

            let handles = builder.get_handle().unwrap();
            builder.insent_bot(addr.to_string(), Arc::new(RefCell::new(bot)));

            tokio::spawn(async move {
                while let Some(msg) = receive.recv().await {
                    let addr_str = msg.addr;

                    if msg.msg.is_close() {
                        builder.remove_bot(&addr_str);
                    }
                    let bot = builder.get_bot(&addr_str);
                    let msg = Bot::message2cq(msg.msg);
                    if bot.is_none() {
                        //error
                    }

                    let mut bot = Arc::clone(bot.unwrap());
                    for f in &handles {
                        let result = f(msg, &mut (*bot).borrow_mut());
                        if result.is_err() {
                            //error
                        }
                    }
                }
            });
        });

        loop {
            let addr_send = bot_in.clone();
            if let Ok((tcp_stream, addr)) = socket.accept().await {
                let (bot_receiver_in, mut bot_receiver_out) = tokio::sync::mpsc::channel::<RecMessage>(5);
                let (bot_sender_in, mut bot_sender_out) = tokio::sync::mpsc::channel::<Message>(5);
                let addr_str = addr.to_string();
                let bot = Bot{
                    addr:Some(addr),
                    msg_sender: Some(bot_sender_in),
                    msg_receiver: Some(bot_receiver_out),
                    handle_vec: None
                };
                addr_send.send(bot).await;
                tokio::spawn(BotBuilder::start_ws_listener(tcp_stream, addr_str, bot_receiver_in.clone(), bot_sender_out));
            }
        }
    }
    async fn start_ws_listener(tcp_stream: TcpStream, addr: String, mut msg_receive: Sender<RecMessage>, mut msg_sende: Receiver<Message>) {
        let ws_stream = tokio_tungstenite::accept_async(tcp_stream)
            .await
            .expect("not websocket");

        let (mut ws_send, mut ws_receive) = ws_stream.split();
        loop {
            tokio::select! {
                msg = ws_receive.next() => {
                    match msg {
                        Some(msg) => {
                            if msg.is_err() {
                                //error
                            }
                            let msg = msg.unwrap();
                            let msg_package = RecMessage{
                                addr: addr.clone(),
                                msg: msg
                            };
                            msg_receive.send(msg_package).await;
                        }
                        None => break,
                    }
                }
                msg = msg_sende.recv() => {
                    if let Some(msg) = msg {
                        ws_send.send(msg).await;
                    }
                }
            }
        }
    }
}

impl Bot {
    pub async fn send(self, message: CQMessage) -> Result<()> {
        Ok(())
    }

    /// todo
    fn cq2message(msg: CQMessage) -> Message {
        Message::Text("".to_string())
    }
    fn message2cq(msg: Message) -> CQMessage {
        CQMessage{}
    }
}