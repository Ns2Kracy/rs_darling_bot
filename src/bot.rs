use crate::driver::driver;

pub struct DarlingBot {
    name: String,
    driver: driver::Driver,
}

impl DarlingBot {
    pub fn new(name: String, driver: driver::Driver) -> Self {
        DarlingBot { name, driver }
    }
}

// 通过匹配选择通信驱动
pub fn new_darling_bot(bot: DarlingBot) {
    match bot.driver {
        driver::Driver::WebsocketServer => {
            // 启动websocket服务器
            // todo
        }
        driver::Driver::WebsocketClient => {
            // 启动websocket客户端
            // todo
        }
        driver::Driver::HttpServer => {
            // 启动http服务器
            // todo
        }
        driver::Driver::HttpClient => {
            // 启动http客户端
            // todo
        }
    }
}
