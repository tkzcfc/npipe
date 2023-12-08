
use serde::Deserialize;


// 身份认证
#[derive(Debug, Deserialize, Clone)]
pub struct AuthenticationRequest
{
    // token
    pub token: String,
    // id
    pub id: String,
}


// 身份认证回复
#[derive(Debug, Deserialize, Clone)]
pub struct AuthenticationResponse
{
    // 0:ok
    pub code: String,
    //
    pub id: String,
}

