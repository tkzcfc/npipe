use super::proto;
use super::support::auth_context;
use crate::global::config::GLOBAL_CONFIG;
use crate::global::GLOBAL_DB_POOL;
use crate::orm_entity::prelude::User;
use actix_identity::Identity;
use actix_web::{error, Error, HttpMessage, HttpRequest, HttpResponse, Responder};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

pub(super) async fn test_auth(identity: Option<Identity>) -> actix_web::Result<impl Responder> {
    match auth_context(identity).await {
        Ok(auth) => Ok(HttpResponse::Ok().json(proto::LoginResponse {
            code: 0,
            msg: "Success".into(),
            role: Some(auth.role),
            user_id: auth.user_id,
            username: auth.username,
        })),
        Err(_) => Ok(HttpResponse::Ok().json(proto::LoginResponse {
            code: 10086,
            msg: "Session expired, please log in again.".into(),
            role: None,
            user_id: None,
            username: None,
        })),
    }
}

pub(super) async fn logout(id: Identity) -> actix_web::Result<HttpResponse, Error> {
    id.logout();
    Ok(HttpResponse::Ok().json(proto::GeneralResponse {
        code: 10086,
        msg: "Session expired, please log in again.".into(),
    }))
}

pub(super) async fn login(
    request: HttpRequest,
    body: String,
) -> actix_web::Result<HttpResponse, Error> {
    let req = serde_json::from_str::<proto::LoginReq>(&body)?;

    // 管理员登录（配置文件中的账号）
    if !GLOBAL_CONFIG.web_username.is_empty()
        && GLOBAL_CONFIG.web_username == req.username
        && GLOBAL_CONFIG.web_password == req.password
    {
        Identity::login(&request.extensions(), "admin".to_owned())?;

        return Ok(HttpResponse::Ok().json(proto::LoginResponse {
            code: 0,
            msg: "Success".into(),
            role: Some("admin".into()),
            user_id: None,
            username: Some(req.username),
        }));
    }

    if let Some(user) = User::find()
        .filter(crate::orm_entity::user::Column::Username.eq(&req.username))
        .filter(crate::orm_entity::user::Column::Password.eq(&req.password))
        .one(GLOBAL_DB_POOL.get().unwrap())
        .await
        .map_err(|err| error::ErrorInternalServerError(format!("sql error:{}", err)))?
    {
        if user.enabled != 1 {
            return Ok(HttpResponse::Ok().json(proto::LoginResponse {
                code: -3,
                msg: "User has been disabled".into(),
                role: None,
                user_id: None,
                username: None,
            }));
        }
        if user.web_access != 1 {
            return Ok(HttpResponse::Ok().json(proto::LoginResponse {
                code: -4,
                msg: "Console access has not been approved".into(),
                role: None,
                user_id: None,
                username: None,
            }));
        }

        Identity::login(&request.extensions(), format!("user:{}", user.id))?;

        return Ok(HttpResponse::Ok().json(proto::LoginResponse {
            code: 0,
            msg: "Success".into(),
            role: Some("user".into()),
            user_id: Some(user.id),
            username: Some(user.username),
        }));
    }

    Ok(HttpResponse::Ok().json(proto::LoginResponse {
        code: -2,
        msg: "Incorrect username or password".into(),
        role: None,
        user_id: None,
        username: None,
    }))
}
