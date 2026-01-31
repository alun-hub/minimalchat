use serde::{Deserialize, Serialize};
use crate::database::Database;
use crate::mattermost::MattermostClient;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub team_id: String,
    #[serde(rename = "type")]
    pub channel_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub channel_id: String,
    pub user_id: String,
    pub username: String,
    pub message: String,
    pub create_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub server_url: String,
    pub auth_token: String,
    pub sync_interval_seconds: u64,
    pub max_messages_per_channel: i32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            server_url: String::new(),
            auth_token: String::new(),
            sync_interval_seconds: 5, // Default: 5 seconds
            max_messages_per_channel: 100,
        }
    }
}

pub struct AppState {
    pub client: Option<MattermostClient>,
    pub db: Database,
    pub settings: Settings,
}
