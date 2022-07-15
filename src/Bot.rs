use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::collections::HashMap;
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
use tokio::sync::mpsc::{Sender, UnboundedSender};

type Bots = HashMap<SocketAddr, Arc<Bot>>;
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
    handle_vec: HandleVec,
}

/// 使用ws-client连接
/// todo
pub struct Bot {
    ws_server: Option<WsStream>,
    handle_vec: Option<HandleVec>,
    addr: Option<SocketAddr>,
}

pub struct CQMessage {}

impl BotBuilder {
    pub fn new() -> Self {
        BotBuilder {
            bots: HashMap::new(),
            handle_vec: Vec::new(),
        }
    }

    /// 可以最多添加 int_max个function
    /// 好像是废话
    pub fn add_handle(&mut self, handle_fn: Handle) {
        self.handle_vec.push(handle_fn);
    }

    /// 启动
    /// 如果想启动任务请使用
    /// ```
    /// tokio::spawn(BotBuilder::start(builder));
    /// ```
    pub async fn start(builder: &mut BotBuilder, addr: &str) {
        let socket;
        socket = TcpListener::bind(&addr).await.expect("bind error");

        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<Message>();

        let (bot_out, mut bot_in) = tokio::sync::mpsc::channel::<Arc<Bot>>(5);

        let handle = builder.handle_vec.clone();
        tokio::spawn(async {
            let bot = bot_in.recv().await;
            if let Some(mut bot) = bot {
                builder.bots.insert(bot.addr.take().unwrap(), bot);
            }
        });

        while let Ok((tcp_stream, addr)) = socket.accept().await {
            tokio::spawn(BotBuilder::start_ws(tcp_stream, addr, sender.clone(), bot_out.clone()));
        }
    }
    async fn start_ws(tcp_stream: TcpStream, addr: SocketAddr, msg_sender: UnboundedSender<Message>, bot_sender: Sender<Arc<Bot>>) {
        let mut ws_stream = tokio_tungstenite::accept_async(tcp_stream)
            .await
            .expect("not websocket");
        let sender = msg_sender;
        let ws_server = Arc::new(RefCell::new(ws_stream));
        let bot = Bot {
            ws_server: Some(Arc::clone(&ws_server)),
            addr: None,
            handle_vec: None,
        };
        let bot = Arc::new(bot);
        let mut interval = tokio::time::interval(Duration::from_millis(1000));

        bot_sender.send(Arc::clone(&bot));
        loop {
            tokio::select! {
                msg = ws_server.next() => {
                    match msg {
                    Some(msg) => {
                        let msg = msg?;
                        if msg.is_text() ||msg.is_binary() {
                            sender.send(msg).await?;
                        } else if msg.is_close() {
                            break;
                        }
                    }
                    None => break,
                    }
                }
                _ = interval.tick() => {
                // do nothing
            }
            }
        }
    }
    async fn bot_lis(bot: &mut WebSocketStream<TcpStream>, sender: UnboundedSender<Message>) {
        let mut ws = bot;
        loop {
            let msg = ws.next();
            match msg.await {
                Some(msg) => {
                    if msg.is_err() {
                        continue;
                    }
                    let msg = msg.unwrap();
                    if msg.is_text() {
                        sender.send(msg);
                    } else if msg.is_close() {
                        break;
                    }
                }
                None => break
            };
        };
    }
}

impl Bot {
    pub async fn send(self, message: CQMessage) -> Result<()> {
        *(self.ws_server.ok_or_else(|| { Error::msg("send error: ws is null") })?)
            .borrow_mut()
            .send(Bot::cq2message(message))
            .await?;
        Ok(())
    }

    /// todo
    fn cq2message(msg: CQMessage) -> Message {
        Message::Text("".to_string())
    }
}