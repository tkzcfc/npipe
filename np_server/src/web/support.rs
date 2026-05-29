use super::proto;
use crate::global::manager::GLOBAL_MANAGER;
use crate::global::GLOBAL_DB_POOL;
use crate::orm_entity::operation_log;
use crate::orm_entity::prelude::User;
use actix_identity::Identity;
use actix_web::{error, Error, HttpResponse};
use chrono::Utc;
use sea_orm::ActiveValue::{NotSet, Set};
use sea_orm::{ActiveModelTrait, EntityTrait};

#[derive(Clone)]
pub(super) struct AuthContext {
    pub(super) role: String,
    pub(super) user_id: Option<u32>,
}

pub(super) async fn auth_context(
    identity: Option<Identity>,
) -> actix_web::Result<AuthContext, Error> {
    let id = match identity.map(|id| id.id()) {
        None => "anonymous".to_owned(),
        Some(Ok(id)) => id,
        Some(Err(err)) => return Err(error::ErrorInternalServerError(err)),
    };

    if id == "anonymous" {
        return Err(error::ErrorUnauthorized("Session expired"));
    }

    if id == "admin" {
        return Ok(AuthContext {
            role: "admin".to_owned(),
            user_id: None,
        });
    }

    if let Some(user_id) = id.strip_prefix("user:").and_then(|it| {
        it.split_once(':')
            .map(|(id, _)| id)
            .unwrap_or(it)
            .parse::<u32>()
            .ok()
    }) {
        let Some(user) = User::find_by_id(user_id)
            .one(GLOBAL_DB_POOL.get().unwrap())
            .await
            .map_err(|err| error::ErrorInternalServerError(format!("sql error:{}", err)))?
        else {
            return Err(error::ErrorUnauthorized("Session expired"));
        };

        if user.enabled != 1 || user.web_access != 1 {
            return Err(error::ErrorUnauthorized("Session expired"));
        }

        return Ok(AuthContext {
            role: "user".to_owned(),
            user_id: Some(user_id),
        });
    }

    Err(error::ErrorUnauthorized("Session expired"))
}

pub(super) fn forbidden_response() -> HttpResponse {
    HttpResponse::Ok().json(proto::GeneralResponse {
        code: 403,
        msg: "Forbidden".into(),
    })
}

pub(super) async fn require_admin(
    identity: Option<Identity>,
) -> actix_web::Result<Result<AuthContext, HttpResponse>, Error> {
    let auth = auth_context(identity).await?;
    if auth.role != "admin" {
        return Ok(Err(forbidden_response()));
    }
    Ok(Ok(auth))
}

pub(super) async fn record_operation(
    action: &str,
    target_type: &str,
    target_id: u32,
    target_name: &str,
    detail: &str,
) {
    let db = match GLOBAL_DB_POOL.get() {
        Some(db) => db,
        None => return,
    };

    let model = operation_log::ActiveModel {
        id: NotSet,
        actor: Set("admin".to_owned()),
        action: Set(action.to_owned()),
        target_type: Set(target_type.to_owned()),
        target_id: Set(target_id),
        target_name: Set(target_name.to_owned()),
        detail: Set(detail.to_owned()),
        created_at: Set(Utc::now().naive_utc()),
    };

    if let Err(err) = model.insert(db).await {
        log::error!("operation log insert error: {}", err);
    }
}

pub(super) async fn player_name(player_id: u32) -> String {
    match User::find_by_id(player_id)
        .one(GLOBAL_DB_POOL.get().unwrap())
        .await
    {
        Ok(Some(user)) => user.username,
        _ => String::new(),
    }
}

pub(super) async fn player_online(player_id: u32) -> bool {
    if player_id == 0 {
        return true;
    }

    if let Some(player) = GLOBAL_MANAGER.player_manager.get_player(player_id) {
        return player.read().await.is_online();
    }

    false
}

pub(super) fn bool_text(value: bool) -> &'static str {
    if value {
        "enabled"
    } else {
        "disabled"
    }
}
