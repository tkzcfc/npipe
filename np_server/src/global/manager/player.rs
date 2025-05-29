use crate::global::GLOBAL_DB_POOL;
use crate::orm_entity::prelude::User;
use crate::orm_entity::user;
use crate::player::{Player, PlayerId};
use crate::utils::str::{is_valid_password, is_valid_username};
use chrono::Utc;
use rand::prelude::StdRng;
use rand::{Rng, SeedableRng};
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};
use std::collections::HashMap;
use std::sync::Arc;

use crate::global::manager::GLOBAL_MANAGER;
use tokio::sync::RwLock;

pub struct PlayerDbData {
    pub id: u32,
    pub username: String,
    pub password: String,
}

pub struct PlayerManager {
    players: RwLock<Vec<Arc<RwLock<Player>>>>,
    player_map: RwLock<HashMap<PlayerId, Arc<RwLock<Player>>>>,
}

impl PlayerManager {
    pub(crate) fn new() -> PlayerManager {
        PlayerManager {
            players: RwLock::new(Vec::new()),
            player_map: RwLock::new(HashMap::new()),
        }
    }

    pub async fn load_all_player(&self) -> anyhow::Result<()> {
        let users = User::find().all(GLOBAL_DB_POOL.get().unwrap()).await?;

        for user in users {
            self.create_player(user.id).await;
        }

        Ok(())
    }

    pub async fn contain(&self, player_id: PlayerId) -> bool {
        self.player_map.read().await.get(&player_id).is_some()
    }

    pub async fn get_player(&self, player_id: PlayerId) -> Option<Arc<RwLock<Player>>> {
        self.player_map.read().await.get(&player_id).cloned()
    }

    pub async fn create_player(&self, player_id: PlayerId) -> Arc<RwLock<Player>> {
        let player = Player::new(player_id);
        self.players.write().await.push(player.clone());
        self.player_map
            .write()
            .await
            .insert(player.read().await.get_player_id(), player.clone());
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

        let mut index_to_find: Option<usize> = None;
        for (index, value) in self.players.read().await.iter().enumerate() {
            if value.read().await.get_player_id() == player_id {
                index_to_find = Some(index);
                break;
            }
        }

        if let Some(index) = index_to_find {
            let player = self.players.write().await.remove(index);
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
        // 参数长度越界检查
        if !is_valid_username(username) || !is_valid_password(password) {
            return Ok((-1, "usernames may not exceed 30 characters, and passwords may not exceed 15 characters.".into()));
        }

        // 执行查询以检查用户名是否存在
        if User::find()
            .filter(user::Column::Username.eq(username))
            .one(GLOBAL_DB_POOL.get().unwrap())
            .await?
            .is_some()
        {
            return Ok((-2, "user already exists".into()));
        }

        let mut rng = StdRng::from_entropy();
        let mut count = 0;
        loop {
            count += 1;
            // 循环次数过多
            if count > 10000 {
                return Ok((-3, "too many cycles".into()));
            }

            // 随机新的玩家id
            let id: u32 = rng.gen_range(10000000..99999999);
            if self.contain(id).await {
                continue;
            }

            let new_user = user::ActiveModel {
                id: Set(id),
                username: Set(username.to_owned()),
                password: Set(password.to_owned()),
                create_time: Set(Utc::now().naive_utc()),
            };

            let _ = new_user.insert(GLOBAL_DB_POOL.get().unwrap()).await?;
            self.create_player(id).await;
            return Ok((0, "".into()));
        }
    }

    /// 更新玩家数据
    pub async fn update_player(&self, data: PlayerDbData) -> anyhow::Result<()> {
        let user = User::find_by_id(data.id)
            .one(GLOBAL_DB_POOL.get().unwrap())
            .await?;
        anyhow::ensure!(user.is_some(), "can't find user: {}", data.id);

        let mut user: user::ActiveModel = user.unwrap().into();
        user.password = Set(data.username.to_owned());
        user.password = Set(data.password.to_owned());

        let _ = user.update(GLOBAL_DB_POOL.get().unwrap()).await?;
        Ok(())
    }
}
