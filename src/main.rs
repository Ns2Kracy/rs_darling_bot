use driver::Driver::HttpClient;
use driver::Driver::HttpServer;
use driver::Driver::WebsocketClient;
use driver::Driver::WebsocketServer;
use rs_darling_bot::bot::DarlingBot;
use rs_darling_bot::driver::driver;

#[tokio::main]
async fn main() {
    let bot = DarlingBot::new(
        "DarlingBot".to_string(),
        WebsocketServer::NewWebsocketServer("ws://127.0.0.1:5700".to_string()),
    );
    rs_darling_bot::bot::new_darling_bot(bot);
}
