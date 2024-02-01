use crate::global::GLOBAL_DB_POOL;
use crate::player::{Player, PlayerId};
use crate::utils::str::{is_valid_password, is_valid_username};
use anyhow::anyhow;
use rand::prelude::StdRng;
use rand::{Rng, SeedableRng};
use sqlx::Row;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct PlayerDbData {
    pub id: u32,
    pub username: String,
    pub password: String,
}

pub struct PlayerManager {
    players: Vec<Arc<RwLock<Player>>>,
    player_map: HashMap<PlayerId, Arc<RwLock<Player>>>,
}

impl PlayerManager {
    pub(crate) fn new() -> PlayerManager {
        PlayerManager {
            players: Vec::new(),
            player_map: HashMap::new(),
        }
    }

    pub async fn load_all_player(&mut self) -> anyhow::Result<()> {
        let query = "SELECT id, type FROM user";
        let rows = sqlx::query(query)
            .fetch_all(crate::global::GLOBAL_DB_POOL.get().unwrap())
            .await?;

        for row in rows {
            let id: u32 = row.get("id");
            let r#type: u8 = row.get("type");
            self.create_player(id, r#type).await;
        }

        Ok(())
    }

    pub fn contain(&self, player_id: PlayerId) -> bool {
        self.player_map.get(&player_id).is_some()
    }

    pub fn get_player(&self, player_id: PlayerId) -> Option<Arc<RwLock<Player>>> {
        if let Some(player) = self.player_map.get(&player_id) {
            Some(player.clone())
        } else {
            None
        }
    }

    pub async fn create_player(
        &mut self,
        player_id: PlayerId,
        player_type: u8,
    ) -> Arc<RwLock<Player>> {
        let player = Player::new(player_id, player_type);
        self.players.push(player.clone());
        self.player_map
            .insert(player.read().await.get_player_id(), player.clone());
        player
    }

    /// 删除玩家
    pub async fn delete_player(&mut self, player_id: u32) -> anyhow::Result<()> {
        if sqlx::query!("DELETE FROM user WHERE id = ?", player_id)
            .execute(GLOBAL_DB_POOL.get().unwrap())
            .await?
            .rows_affected()
            == 1
        {
            let mut index_to_find: Option<usize> = None;
            for (index, value) in self.players.iter().enumerate() {
                if value.read().await.get_player_id() == player_id {
                    index_to_find = Some(index);
                    break;
                }
            }

            if let Some(index) = index_to_find {
                let player = self.players.remove(index);
                player.write().await.close_session();
            }

            return Ok(());
        }
        Err(anyhow!(format!("Unable to find player: {}", player_id)))
    }

    /// 新加玩家
    pub async fn add_player(
        &mut self,
        username: &String,
        password: &String,
    ) -> anyhow::Result<(i32, String)> {
        // 参数长度越界检查
        if !is_valid_username(username) || !is_valid_password(password) {
            return Ok((-1, "Bad parameter".into()));
        }

        // 执行查询以检查用户名是否存在
        let record = sqlx::query!(
            "SELECT EXISTS(SELECT 1 FROM user WHERE username = ?) as 'exists'",
            username
        )
        .fetch_one(GLOBAL_DB_POOL.get().unwrap())
        .await?;

        // 用户已存在
        if record.exists != 0 {
            return Ok((-2, "User already exists".into()));
        }

        let mut rng = StdRng::from_entropy();
        let mut count = 0;
        loop {
            count += 1;
            // 循环次数过多
            if count > 10000 {
                return Ok((-3, "Too many cycles".into()));
            }

            // 随机新的玩家id
            let id: u32 = rng.gen_range(10000000..99999999);
            if self.contain(id) {
                continue;
            }

            return if sqlx::query!(
                "INSERT INTO user (id, username, password, type) VALUES (?, ?, ?, ?)",
                id,
                username,
                password,
                0
            )
            .execute(GLOBAL_DB_POOL.get().unwrap())
            .await?
            .rows_affected()
                == 1
            {
                self.create_player(id, 0).await;
                Ok((0, "".into()))
            } else {
                Ok((-4, "sqlx error".into()))
            };
        }
    }

    /// 更新玩家数据
    pub async fn update_player(&self, data: PlayerDbData) -> anyhow::Result<()> {
        if sqlx::query!(
            "UPDATE user SET username = ?, password = ? WHERE id = ?",
            data.username,
            data.password,
            data.id
        )
        .execute(GLOBAL_DB_POOL.get().unwrap())
        .await?
        .rows_affected()
            == 1
        {
            return Ok(());
        }
        return Err(anyhow!(format!(
            "Data update failed, player_id: {}",
            data.id
        )));
    }
}
