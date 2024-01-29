use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct LoginReq {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct LoginAck {
    pub msg: String,
    pub code: i32,
}


#[derive(Serialize, Deserialize)]
pub struct LogoutAck {
}
