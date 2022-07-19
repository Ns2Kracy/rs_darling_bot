use async_trait::async_trait;
/// 四种通信驱动器:
/// 1. websocket_server
/// 2. websocket_client
/// 3. http_server
/// 4. http_client
pub enum Driver {
    WebsocketServer,
    WebsocketClient,
    HttpServer,
    HttpClient,
}
// 实现Driver接口
#[async_trait]
pub trait DriverInterface {
    // 启动驱动器
    async fn start(&self) -> Result<(), Box<dyn std::error::Error>>;
    // 停止驱动器
    async fn stop(&self) -> Result<(), Box<dyn std::error::Error>>;
}
// 实现Driver接口
impl DriverInterface for Driver {
    fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        // todo
        Ok(())
    }
    fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        // todo
        Ok(())
    }
}
