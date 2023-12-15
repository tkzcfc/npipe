use crate::session::Session;
use np_base::client_server;
use np_base::message_map::MessageType;
use std::io;

impl Session {
    pub async fn on_recv_message(&mut self, serial: i32, message: &MessageType) -> io::Result<()> {
        match message {
            MessageType::ClientServerLoginReq(msg) => {
                return self.on_login_requst(serial, msg).await
            }
            _ => {}
        }
        Ok(())
    }

    async fn on_login_requst(
        &mut self,
        serial: i32,
        message: &client_server::LoginReq,
    ) -> io::Result<()> {
        if self.player.is_some() {
            // 重复发送登录请求
            return Ok(());
        }

        // 根据用户名查找用户id
        let player_id = 100u32;

        Ok(())
    }
}
