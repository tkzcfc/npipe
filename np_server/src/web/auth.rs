use super::proto;
use super::rate_limit;
use super::support::auth_context;
use crate::global::config::GLOBAL_CONFIG;
use crate::global::GLOBAL_DB_POOL;
use crate::orm_entity::login_history;
use crate::orm_entity::prelude::User;
use actix_identity::Identity;
use actix_web::{error, Error, HttpMessage, HttpRequest, HttpResponse, Responder};
use chrono::Utc;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, NotSet, QueryFilter};

/// 向 login_history 插入一条 web 登录记录（不阻塞主流程）
async fn record_web_login(user_id: u32, ip_addr: String, success: u8) {
    let db = GLOBAL_DB_POOL.get().unwrap();
    let record = login_history::ActiveModel {
        id: NotSet,
        user_id: Set(user_id),
        ip_addr: Set(ip_addr),
        login_time: Set(Utc::now().naive_utc()),
        logout_time: Set(None),
        duration_secs: Set(None),
        login_source: Set("web".to_owned()),
        success: Set(success),
    };
    if let Err(e) = record.insert(db).await {
        log::error!("record_web_login insert failed: {}", e);
    }
}

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

    let ip_addr = request
        .connection_info()
        .realip_remote_addr()
        .unwrap_or("")
        .to_owned();

    // 登录限速：同一 IP+用户名短时间内失败过多则锁定，抵御在线爆破
    let rl_key = format!("{ip_addr}|{}", req.username);
    if let Some(secs) = rate_limit::locked_secs_remaining(&rl_key) {
        return Ok(HttpResponse::TooManyRequests().json(proto::LoginResponse {
            code: -5,
            msg: format!("Too many failed attempts, try again in {secs}s"),
            role: None,
            user_id: None,
            username: None,
        }));
    }

    // 管理员登录（配置文件中的账号）
    if !GLOBAL_CONFIG.web_username.is_empty()
        && GLOBAL_CONFIG.web_username == req.username
        && crate::utils::password::constant_time_eq(&GLOBAL_CONFIG.web_password, &req.password)
    {
        Identity::login(&request.extensions(), "admin".to_owned())?;
        rate_limit::record_success(&rl_key);
        record_web_login(0, ip_addr, 1).await;

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
        .one(GLOBAL_DB_POOL.get().unwrap())
        .await
        .map_err(|err| error::ErrorInternalServerError(format!("sql error:{}", err)))?
        .filter(|user| crate::utils::password::verify_password(&req.password, &user.password))
    {
        if user.enabled != 1 {
            record_web_login(user.id, ip_addr, 0).await;
            return Ok(HttpResponse::Ok().json(proto::LoginResponse {
                code: -3,
                msg: "User has been disabled".into(),
                role: None,
                user_id: None,
                username: None,
            }));
        }
        if user.web_access != 1 {
            record_web_login(user.id, ip_addr, 0).await;
            return Ok(HttpResponse::Ok().json(proto::LoginResponse {
                code: -4,
                msg: "Console access has not been approved".into(),
                role: None,
                user_id: None,
                username: None,
            }));
        }

        Identity::login(&request.extensions(), format!("user:{}", user.id))?;
        rate_limit::record_success(&rl_key);
        record_web_login(user.id, ip_addr, 1).await;

        return Ok(HttpResponse::Ok().json(proto::LoginResponse {
            code: 0,
            msg: "Success".into(),
            role: Some("user".into()),
            user_id: Some(user.id),
            username: Some(user.username),
        }));
    }

    // 用户名或密码错误（找不到匹配的用户），以 user_id=0 记录
    rate_limit::record_failure(&rl_key);
    record_web_login(0, ip_addr, 0).await;
    Ok(HttpResponse::Ok().json(proto::LoginResponse {
        code: -2,
        msg: "Incorrect username or password".into(),
        role: None,
        user_id: None,
        username: None,
    }))
}
