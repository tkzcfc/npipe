use crate::global::GLOBAL_DB_POOL;
use crate::orm_entity::prelude::User;
use crate::orm_entity::user;
use crate::player::{Player, PlayerId};
use crate::utils::str::{is_valid_password, is_valid_username};
use chrono::Utc;
use dashmap::DashMap;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};
use std::sync::Arc;

use crate::global::manager::GLOBAL_MANAGER;
use tokio::sync::RwLock;

pub struct PlayerManager {
    pub(crate) player_map: DashMap<PlayerId, Arc<RwLock<Player>>>,
}

impl PlayerManager {
    pub(crate) fn new() -> PlayerManager {
        PlayerManager {
            player_map: DashMap::new(),
        }
    }

    pub async fn load_all_player(&self) -> anyhow::Result<()> {
        let users = User::find().all(GLOBAL_DB_POOL.get().unwrap()).await?;
        for user in users {
            self.create_player(user.id);
        }
        Ok(())
    }

    /// 纯 DashMap 查询，无需 async。
    pub fn contain(&self, player_id: PlayerId) -> bool {
        self.player_map.contains_key(&player_id)
    }

    /// 纯 DashMap 查询，无需 async。
    pub fn get_player(&self, player_id: PlayerId) -> Option<Arc<RwLock<Player>>> {
        self.player_map.get(&player_id).map(|r| r.clone())
    }

    /// 纯 DashMap 插入，无需 async。
    pub fn create_player(&self, player_id: PlayerId) -> Arc<RwLock<Player>> {
        let player = Player::new(player_id);
        self.player_map.insert(player_id, player.clone());
        player
    }

    /// 删除玩家
    pub async fn delete_player(&self, player_id: u32) -> anyhow::Result<()> {
        let player_tunnels: Vec<_> = GLOBAL_MANAGER
            .tunnel_manager
            .tunnels
            .read()
            .await
            .iter()
            .filter_map(|x| {
                if x.sender == player_id || x.receiver == player_id {
                    Some(x.id)
                } else {
                    None
                }
            })
            .collect();

        for tunnel_id in player_tunnels {
            GLOBAL_MANAGER
                .tunnel_manager
                .delete_tunnel(tunnel_id)
                .await?;
        }

        let db = GLOBAL_DB_POOL.get().unwrap();
        let rows_affected = User::delete_by_id(player_id).exec(db).await?.rows_affected;
        anyhow::ensure!(
            rows_affected == 1,
            "delete_player: rows_affected = {}",
            rows_affected
        );

        // DashMap::remove: O(1)，只锁对应 shard，无需遍历
        if let Some((_, player)) = self.player_map.remove(&player_id) {
            player.write().await.close_session();
        }

        Ok(())
    }

    /// 新加玩家
    pub async fn add_player(
        &self,
        username: &String,
        password: &String,
    ) -> anyhow::Result<(i32, String)> {
        if !is_valid_username(username) || !is_valid_password(password) {
            return Ok((-1, "usernames may not exceed 30 characters, and passwords may not exceed 15 characters.".into()));
        }

        if User::find()
            .filter(user::Column::Username.eq(username))
            .one(GLOBAL_DB_POOL.get().unwrap())
            .await?
            .is_some()
        {
            return Ok((-2, "user already exists".into()));
        }

        let mut count = 0;
        loop {
            count += 1;
            if count > 10000 {
                return Ok((-3, "too many cycles".into()));
            }

            let id: u32 = rand::random_range(10000000..99999999);
            if self.contain(id) {
                continue;
            }

            let new_user = user::ActiveModel {
                id: Set(id),
                username: Set(username.to_owned()),
                password: Set(password.to_owned()),
                create_time: Set(Utc::now().naive_utc()),
                enabled: Set(1),
                web_access: Set(0),
            };

            let _ = new_user.insert(GLOBAL_DB_POOL.get().unwrap()).await?;
            self.create_player(id);
            return Ok((0, "".into()));
        }
    }

    /// 修改玩家用户名
    pub async fn rename_player(&self, player_id: u32, username: &String) -> anyhow::Result<()> {
        anyhow::ensure!(is_valid_username(username), "username format error");

        if let Some(existing_user) = User::find()
            .filter(user::Column::Username.eq(username))
            .one(GLOBAL_DB_POOL.get().unwrap())
            .await?
        {
            anyhow::ensure!(existing_user.id == player_id, "user already exists");
        }

        let user = User::find_by_id(player_id)
            .one(GLOBAL_DB_POOL.get().unwrap())
            .await?;
        anyhow::ensure!(user.is_some(), "can't find user: {}", player_id);

        let mut user: user::ActiveModel = user.unwrap().into();
        user.username = Set(username.to_owned());

        let _ = user.update(GLOBAL_DB_POOL.get().unwrap()).await?;
        Ok(())
    }

    /// 重置玩家密码
    pub async fn reset_player_password(
        &self,
        player_id: u32,
        password: &String,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(is_valid_password(password), "password format error");

        let user = User::find_by_id(player_id)
            .one(GLOBAL_DB_POOL.get().unwrap())
            .await?;
        anyhow::ensure!(user.is_some(), "can't find user: {}", player_id);

        let mut user: user::ActiveModel = user.unwrap().into();
        user.password = Set(password.to_owned());

        let _ = user.update(GLOBAL_DB_POOL.get().unwrap()).await?;
        Ok(())
    }

    /// 修改玩家启用状态
    pub async fn update_player_status(&self, player_id: u32, enabled: u8) -> anyhow::Result<()> {
        let user = User::find_by_id(player_id)
            .one(GLOBAL_DB_POOL.get().unwrap())
            .await?;
        anyhow::ensure!(user.is_some(), "can't find user: {}", player_id);

        let mut user: user::ActiveModel = user.unwrap().into();
        user.enabled = Set(enabled);
        let _ = user.update(GLOBAL_DB_POOL.get().unwrap()).await?;

        if enabled == 0 {
            if let Some(player) = self.get_player(player_id) {
                player.write().await.kick_offline();
            }
        }

        Ok(())
    }

    /// 修改玩家后台访问权限
    pub async fn update_player_web_access(
        &self,
        player_id: u32,
        web_access: u8,
    ) -> anyhow::Result<()> {
        let user = User::find_by_id(player_id)
            .one(GLOBAL_DB_POOL.get().unwrap())
            .await?;
        anyhow::ensure!(user.is_some(), "can't find user: {}", player_id);

        let mut user: user::ActiveModel = user.unwrap().into();
        user.web_access = Set(web_access);
        let _ = user.update(GLOBAL_DB_POOL.get().unwrap()).await?;

        Ok(())
    }
}
