use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use anyhow::{Error, Result};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;
use futures_util::{SinkExt, StreamExt, TryStreamExt};
use futures_util::stream::Next;
use tokio::sync::mpsc::UnboundedSender;

type Bots = HashMap<SocketAddr, Arc<Bot>>;
type Handle=fn(CQMessage, &mut Bot) -> Result<()>;
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
pub struct Bot{
    ws_server: Option<WsStream>,
    handle_vec: Option<HandleVec>,
    addr: Option<SocketAddr>,
}

pub struct CQMessage {

}

impl BotBuilder {
    pub fn new() -> Self{
        BotBuilder{
            bots:HashMap::new(),
            handle_vec:Vec::new()
        }
    }

    /// 可以最多添加 int_max个function
    /// 好像是废话
    pub fn add_handle(&mut self, handle_fn: Handle) {
        self.handle_vec.push(handle_fn);
    }

    /// 启动
    /// 如果想阻塞任务请使用.await
    /// ```
    /// builder.start().await;
    /// ```
    /// 不想阻塞任务可以使用
    /// ```
    /// tokio::spawn(builder.start());
    /// ```
    pub async fn start(&mut self, addr: &str) {
        let socket;
        socket = TcpListener::bind(&addr).await.expect("bind error");

        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<Message>();

        while let Ok((tcp_stream, addr)) = socket.accept().await {
            let mut ws_stream = tokio_tungstenite::accept_async(tcp_stream)
                .await
                .expect("not websocket");
            let sender = sender.clone();
            let ws_server = Arc::new(RefCell::new(ws_stream));
            let bot = Bot {
                ws_server: Some(Arc::clone(&ws_server)),
                addr: None,
                handle_vec: None
            };
            let bot = Arc::new(bot);
            self.bots.insert(addr, Arc::clone(&bot));
            tokio::spawn(BotBuilder::bot_lis(Arc::clone(&bot), sender));

        }
    }

    async fn bot_lis(bot:Arc<Bot>, sender: UnboundedSender<Message>){
        let mut ws_ref = bot.ws_server.unwrap();
        let mut ws = (*ws_ref).borrow_mut();
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
                },
                None => break
            };
        };
    }
}

impl Bot {
    pub async fn send(self, message: CQMessage) -> Result<()>{
        *(self.ws_server.ok_or_else(||{Error::msg("send error: ws is null")})?)
            .borrow_mut()
            .send(Bot::cq2message(message))
            .await?;
        Ok(())
    }

    /// todo
    fn cq2message(msg : CQMessage) -> Message {
        Message::Text("".to_string())
    }
}