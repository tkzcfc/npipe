use super::proto;
use super::support::{
    auth_context, bool_text, forbidden_response, player_online, record_operation, AuthContext,
};
use crate::global::manager::GLOBAL_MANAGER;
use crate::global::GLOBAL_DB_POOL;
use crate::orm_entity::prelude::User;
use crate::orm_entity::tunnel;
use crate::orm_entity::user;
use crate::utils::str::{
    get_tunnel_address_port, is_valid_tunnel_endpoint_address, is_valid_tunnel_source_address,
};
use actix_identity::Identity;
use actix_web::{HttpResponse, Responder};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use std::collections::HashMap;

pub(super) async fn tunnel_list(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    let auth = auth_context(identity).await?;

    let req = serde_json::from_str::<proto::TunnelListRequest>(&body)?;

    let (tunnel_list, total_count) = if auth.role == "admin" {
        GLOBAL_MANAGER
            .tunnel_manager
            .query_with_total(req.page_number, req.page_size)
            .await
    } else if let Some(user_id) = auth.user_id {
        let page_size = if req.page_size == 0 {
            20
        } else {
            req.page_size.min(100)
        };
        let tunnels: Vec<_> = GLOBAL_MANAGER
            .tunnel_manager
            .tunnels
            .read()
            .await
            .iter()
            .filter(|data| data.sender == user_id || data.receiver == user_id)
            .cloned()
            .collect();
        let total_count = tunnels.len();
        let start = req.page_number * page_size;
        let end = (start + page_size).min(total_count);
        let page = if start <= end {
            tunnels[start..end].to_vec()
        } else {
            vec![]
        };
        (page, total_count)
    } else {
        (vec![], 0)
    };

    // 收集所有涉及的用户 ID（排除 0 代表服务器），批量查一次用户名
    let user_ids: Vec<u32> = {
        let mut ids: std::collections::HashSet<u32> = std::collections::HashSet::new();
        for data in &tunnel_list {
            if data.sender != 0 {
                ids.insert(data.sender);
            }
            if data.receiver != 0 {
                ids.insert(data.receiver);
            }
        }
        ids.into_iter().collect()
    };
    let user_name_map: HashMap<u32, String> = if user_ids.is_empty() {
        HashMap::new()
    } else {
        User::find()
            .filter(user::Column::Id.is_in(user_ids))
            .all(GLOBAL_DB_POOL.get().unwrap())
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|u| (u.id, u.username))
            .collect()
    };

    let mut tunnels: Vec<proto::TunnelListItem> = Vec::new();

    for data in tunnel_list {
        let custom_mapping: HashMap<String, String> =
            serde_json::from_str(&data.custom_mapping).map_or(HashMap::new(), |x| x);
        let sender_online = player_online(data.sender).await;
        let receiver_online = player_online(data.receiver).await;
        let available = data.enabled == 1 && sender_online && receiver_online;
        let sender_name = user_name_map.get(&data.sender).cloned().unwrap_or_default();
        let receiver_name = user_name_map
            .get(&data.receiver)
            .cloned()
            .unwrap_or_default();

        tunnels.push(proto::TunnelListItem {
            id: data.id,
            source: data.source,
            endpoint: data.endpoint,
            enabled: data.enabled == 1,
            sender: data.sender,
            receiver: data.receiver,
            sender_name,
            receiver_name,
            description: data.description,
            tunnel_type: data.tunnel_type,
            username: data.username,
            is_compressed: data.is_compressed == 1,
            encryption_method: data.encryption_method,
            custom_mapping,
            sender_online,
            receiver_online,
            available,
        })
    }

    Ok(HttpResponse::Ok().json(proto::TunnelListResponse {
        tunnels,
        cur_page_number: req.page_number,
        total_count,
    }))
}

fn user_tunnel_allowed(auth: &AuthContext, sender: u32, receiver: u32) -> bool {
    auth.role == "admin"
        || auth
            .user_id
            .is_some_and(|user_id| sender == user_id && (receiver == 0 || receiver == user_id))
}

async fn user_can_manage_tunnel(auth: &AuthContext, tunnel_id: u32) -> bool {
    if auth.role == "admin" {
        return true;
    }

    let Some(user_id) = auth.user_id else {
        return false;
    };

    GLOBAL_MANAGER
        .tunnel_manager
        .tunnels
        .read()
        .await
        .iter()
        .any(|it| {
            it.id == tunnel_id
                && it.sender == user_id
                && (it.receiver == 0 || it.receiver == user_id)
        })
}

pub(super) async fn tunnel_detail(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    let auth = auth_context(identity).await?;

    let req = serde_json::from_str::<proto::TunnelDetailRequest>(&body)?;
    if auth.role != "admin" && !user_can_manage_tunnel(&auth, req.id).await {
        return Ok(forbidden_response());
    }
    let tunnel = GLOBAL_MANAGER
        .tunnel_manager
        .tunnels
        .read()
        .await
        .iter()
        .find(|it| it.id == req.id)
        .map(|data| {
            let custom_mapping: HashMap<String, String> =
                serde_json::from_str(&data.custom_mapping).map_or(HashMap::new(), |x| x);

            proto::TunnelDetailItem {
                id: data.id,
                source: data.source.clone(),
                endpoint: data.endpoint.clone(),
                enabled: data.enabled == 1,
                sender: data.sender,
                receiver: data.receiver,
                description: data.description.clone(),
                tunnel_type: data.tunnel_type,
                password: data.password.clone(),
                username: data.username.clone(),
                is_compressed: data.is_compressed == 1,
                encryption_method: data.encryption_method.clone(),
                custom_mapping,
                sender_online: false,
                receiver_online: false,
                available: false,
            }
        });
    let tunnel = if let Some(mut tunnel) = tunnel {
        tunnel.sender_online = player_online(tunnel.sender).await;
        tunnel.receiver_online = player_online(tunnel.receiver).await;
        tunnel.available = tunnel.enabled && tunnel.sender_online && tunnel.receiver_online;
        Some(tunnel)
    } else {
        None
    };

    Ok(HttpResponse::Ok().json(proto::TunnelDetailResponse { tunnel }))
}

pub(super) async fn remove_tunnel(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    let auth = auth_context(identity).await?;

    let req = serde_json::from_str::<proto::TunnelRemoveReq>(&body)?;
    if !user_can_manage_tunnel(&auth, req.id).await {
        return Ok(forbidden_response());
    }
    let old_tunnel = GLOBAL_MANAGER
        .tunnel_manager
        .tunnels
        .read()
        .await
        .iter()
        .find(|item| item.id == req.id)
        .cloned();
    match GLOBAL_MANAGER.tunnel_manager.delete_tunnel(req.id).await {
        Ok(()) => {
            record_operation(
                "remove_tunnel",
                "tunnel",
                req.id,
                &old_tunnel
                    .as_ref()
                    .map(|item| format!("#{} {}", item.id, item.source))
                    .unwrap_or_else(|| format!("#{}", req.id)),
                &old_tunnel
                    .as_ref()
                    .map(build_tunnel_snapshot)
                    .unwrap_or_else(|| "old tunnel not found".to_owned()),
            )
            .await;
            Ok(HttpResponse::Ok().json(proto::GeneralResponse {
                code: 0,
                msg: "Success".into(),
            }))
        }
        Err(err) => Ok(HttpResponse::Ok().json(proto::GeneralResponse {
            code: -1,
            msg: err.to_string(),
        })),
    }
}

pub(super) async fn add_tunnel(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    let auth = auth_context(identity).await?;

    let req = serde_json::from_str::<proto::TunnelAddReq>(&body)?;
    if !user_tunnel_allowed(&auth, req.sender, req.receiver) {
        return Ok(forbidden_response());
    }
    let mut new_tunnel = tunnel::Model {
        source: req.source,
        endpoint: req.endpoint,
        id: 0,
        enabled: req.enabled,
        sender: req.sender,
        receiver: req.receiver,
        description: req.description,
        tunnel_type: req.tunnel_type,
        password: req.password,
        username: req.username,
        is_compressed: req.is_compressed,
        custom_mapping: serde_json::to_string(&req.custom_mapping).map_or("".to_string(), |x| x),
        encryption_method: req.encryption_method,
    };
    let source = new_tunnel.source.clone();
    match GLOBAL_MANAGER
        .tunnel_manager
        .add_tunnel(new_tunnel.clone())
        .await
    {
        Ok(tunnel_id) => {
            new_tunnel.id = tunnel_id;
            record_operation(
                "add_tunnel",
                "tunnel",
                tunnel_id,
                &format!("#{} {}", tunnel_id, source),
                &build_tunnel_snapshot(&new_tunnel),
            )
            .await;
            Ok(HttpResponse::Ok().json(proto::GeneralResponse {
                code: 0,
                msg: "Success".into(),
            }))
        }
        Err(err) => Ok(HttpResponse::Ok().json(proto::GeneralResponse {
            code: -1,
            msg: err.to_string(),
        })),
    }
}

fn tunnel_type_name(tunnel_type: u32) -> &'static str {
    match tunnel_type {
        0 => "TCP",
        1 => "UDP",
        2 => "SOCKS5",
        3 => "HTTP",
        _ => "Unknown",
    }
}

fn push_change<T: std::fmt::Display + PartialEq>(
    changes: &mut Vec<String>,
    label: &str,
    old: T,
    new: T,
) {
    if old != new {
        changes.push(format!("{}: {} -> {}", label, old, new));
    }
}

fn build_tunnel_update_detail(old: &tunnel::Model, new: &tunnel::Model) -> String {
    let mut changes = Vec::new();

    push_change(&mut changes, "source", &old.source, &new.source);
    push_change(&mut changes, "endpoint", &old.endpoint, &new.endpoint);
    push_change(
        &mut changes,
        "enabled",
        bool_text(old.enabled == 1),
        bool_text(new.enabled == 1),
    );
    push_change(&mut changes, "sender", old.sender, new.sender);
    push_change(&mut changes, "receiver", old.receiver, new.receiver);
    push_change(
        &mut changes,
        "type",
        tunnel_type_name(old.tunnel_type),
        tunnel_type_name(new.tunnel_type),
    );
    push_change(&mut changes, "username", &old.username, &new.username);
    if old.password != new.password {
        changes.push("password: changed".to_owned());
    }
    push_change(
        &mut changes,
        "compression",
        bool_text(old.is_compressed == 1),
        bool_text(new.is_compressed == 1),
    );
    push_change(
        &mut changes,
        "encryption",
        &old.encryption_method,
        &new.encryption_method,
    );
    push_change(
        &mut changes,
        "mapping",
        &old.custom_mapping,
        &new.custom_mapping,
    );
    push_change(
        &mut changes,
        "description",
        &old.description,
        &new.description,
    );

    if changes.is_empty() {
        "no changes".to_owned()
    } else {
        changes.join("; ")
    }
}

fn build_tunnel_snapshot(tunnel: &tunnel::Model) -> String {
    let mut parts = vec![
        format!("source: {}", tunnel.source),
        format!("endpoint: {}", tunnel.endpoint),
        format!("enabled: {}", bool_text(tunnel.enabled == 1)),
        format!("sender: {}", tunnel.sender),
        format!("receiver: {}", tunnel.receiver),
        format!("type: {}", tunnel_type_name(tunnel.tunnel_type)),
        format!("username: {}", tunnel.username),
        format!(
            "password: {}",
            if tunnel.password.is_empty() {
                "empty"
            } else {
                "set"
            }
        ),
        format!("compression: {}", bool_text(tunnel.is_compressed == 1)),
        format!("encryption: {}", tunnel.encryption_method),
    ];

    if !tunnel.custom_mapping.is_empty() && tunnel.custom_mapping != "{}" {
        parts.push(format!("mapping: {}", tunnel.custom_mapping));
    }
    if !tunnel.description.is_empty() {
        parts.push(format!("description: {}", tunnel.description));
    }

    parts.join("; ")
}

pub(super) async fn update_tunnel(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    let auth = auth_context(identity).await?;

    let req = serde_json::from_str::<proto::TunnelUpdateReq>(&body)?;
    if !user_can_manage_tunnel(&auth, req.id).await
        || !user_tunnel_allowed(&auth, req.sender, req.receiver)
    {
        return Ok(forbidden_response());
    }
    let source = req.source.clone();
    let old_tunnel = GLOBAL_MANAGER
        .tunnel_manager
        .tunnels
        .read()
        .await
        .iter()
        .find(|item| item.id == req.id)
        .cloned();
    let new_tunnel = tunnel::Model {
        source: req.source,
        endpoint: req.endpoint,
        id: req.id,
        enabled: req.enabled,
        sender: req.sender,
        receiver: req.receiver,
        description: req.description,
        tunnel_type: req.tunnel_type,
        password: req.password,
        username: req.username,
        is_compressed: req.is_compressed,
        custom_mapping: serde_json::to_string(&req.custom_mapping).map_or("".to_string(), |x| x),
        encryption_method: req.encryption_method,
    };
    let mut log_tunnel = new_tunnel.clone();
    if req.preserve_password.unwrap_or(false) && log_tunnel.password.is_empty() {
        if let Some(old) = &old_tunnel {
            log_tunnel.password = old.password.clone();
        }
    }
    let detail = old_tunnel
        .as_ref()
        .map(|old| build_tunnel_update_detail(old, &log_tunnel))
        .unwrap_or_else(|| "old tunnel not found".to_owned());

    match GLOBAL_MANAGER
        .tunnel_manager
        .update_tunnel(new_tunnel, req.preserve_password.unwrap_or(false))
        .await
    {
        Ok(()) => {
            record_operation(
                "update_tunnel",
                "tunnel",
                req.id,
                &format!("#{} {}", req.id, source),
                &detail,
            )
            .await;
            Ok(HttpResponse::Ok().json(proto::GeneralResponse {
                code: 0,
                msg: "Success".into(),
            }))
        }
        Err(err) => Ok(HttpResponse::Ok().json(proto::GeneralResponse {
            code: -1,
            msg: err.to_string(),
        })),
    }
}

pub(super) async fn update_tunnel_status(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    let auth = auth_context(identity).await?;

    let req = serde_json::from_str::<proto::TunnelStatusUpdateReq>(&body)?;
    if !user_can_manage_tunnel(&auth, req.id).await {
        return Ok(forbidden_response());
    }
    let old_enabled = GLOBAL_MANAGER
        .tunnel_manager
        .tunnels
        .read()
        .await
        .iter()
        .find(|item| item.id == req.id)
        .map(|item| item.enabled);
    if let Err(err) = GLOBAL_MANAGER
        .tunnel_manager
        .update_tunnel_status(req.id, req.enabled)
        .await
    {
        Ok(HttpResponse::Ok().json(proto::GeneralResponse {
            code: -1,
            msg: err.to_string(),
        }))
    } else {
        let detail = old_enabled
            .map(|old| {
                format!(
                    "enabled: {} -> {}",
                    bool_text(old == 1),
                    bool_text(req.enabled == 1)
                )
            })
            .unwrap_or_else(|| format!("enabled: unknown -> {}", bool_text(req.enabled == 1)));
        record_operation(
            "update_tunnel_status",
            "tunnel",
            req.id,
            &format!("#{}", req.id),
            &detail,
        )
        .await;
        Ok(HttpResponse::Ok().json(proto::GeneralResponse {
            code: 0,
            msg: "Success".into(),
        }))
    }
}

pub(super) async fn tunnel_diagnose(
    identity: Option<Identity>,
    body: String,
) -> actix_web::Result<impl Responder> {
    let auth = auth_context(identity).await?;

    let req = serde_json::from_str::<proto::TunnelDiagnoseRequest>(&body)?;
    if !user_tunnel_allowed(&auth, req.sender, req.receiver) {
        return Ok(forbidden_response());
    }
    if let Some(id) = req.id {
        if !user_can_manage_tunnel(&auth, id).await {
            return Ok(forbidden_response());
        }
    }
    let mut items = Vec::new();

    push_diagnose(
        &mut items,
        "source",
        is_valid_tunnel_source_address(&req.source),
        "Source address is valid",
        "Source address format error",
    );

    let needs_endpoint = matches!(req.tunnel_type, 0 | 1);
    if needs_endpoint {
        push_diagnose(
            &mut items,
            "endpoint",
            is_valid_tunnel_endpoint_address(&req.endpoint),
            "Endpoint address is valid",
            "Endpoint address format error",
        );
    } else {
        items.push(proto::TunnelDiagnoseItem {
            key: "endpoint".to_owned(),
            level: "ok".to_owned(),
            message: "Proxy tunnel does not require endpoint".to_owned(),
        });
    }

    let sender_exists = req.sender == 0 || GLOBAL_MANAGER.player_manager.contain(req.sender);
    let receiver_exists = req.receiver == 0 || GLOBAL_MANAGER.player_manager.contain(req.receiver);
    push_diagnose(
        &mut items,
        "sender",
        sender_exists,
        "Sender exists",
        "Sender player does not exist",
    );
    push_diagnose(
        &mut items,
        "receiver",
        receiver_exists,
        "Receiver exists",
        "Receiver player does not exist",
    );

    let sender_online = player_online(req.sender).await;
    let receiver_online = player_online(req.receiver).await;
    push_runtime_diagnose(&mut items, "sender_online", sender_online, req.sender);
    push_runtime_diagnose(&mut items, "receiver_online", receiver_online, req.receiver);

    let port_conflict = GLOBAL_MANAGER
        .tunnel_manager
        .has_port_conflict(
            req.receiver,
            get_tunnel_address_port(&req.source),
            req.id,
            req.tunnel_type == 1,
        )
        .await;
    push_diagnose(
        &mut items,
        "port",
        !port_conflict,
        "Listen port is available",
        "Listen port already in use",
    );

    let ok = items.iter().all(|item| item.level != "error");
    Ok(HttpResponse::Ok().json(proto::TunnelDiagnoseResponse { ok, items }))
}

fn push_diagnose(
    items: &mut Vec<proto::TunnelDiagnoseItem>,
    key: &str,
    ok: bool,
    ok_message: &str,
    error_message: &str,
) {
    items.push(proto::TunnelDiagnoseItem {
        key: key.to_owned(),
        level: if ok { "ok" } else { "error" }.to_owned(),
        message: if ok { ok_message } else { error_message }.to_owned(),
    });
}

fn push_runtime_diagnose(
    items: &mut Vec<proto::TunnelDiagnoseItem>,
    key: &str,
    online: bool,
    player_id: u32,
) {
    let (level, message) = if player_id == 0 {
        ("ok", "Server endpoint is available")
    } else if online {
        ("ok", "Player is online")
    } else {
        ("warn", "Player is offline now")
    };

    items.push(proto::TunnelDiagnoseItem {
        key: key.to_owned(),
        level: level.to_owned(),
        message: message.to_owned(),
    });
}
