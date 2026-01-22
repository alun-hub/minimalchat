use crate::models::{Channel, Message, Settings};
use rusqlite::{Connection, Result};
use std::sync::{Arc, Mutex};

pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        // Create tables
        conn.execute(
            "CREATE TABLE IF NOT EXISTS channels (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                display_name TEXT NOT NULL,
                team_id TEXT NOT NULL,
                channel_type TEXT NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                channel_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                message TEXT NOT NULL,
                create_at INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_messages_channel_id
             ON messages(channel_id, create_at DESC)",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS message_queue (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                channel_id TEXT NOT NULL,
                message TEXT NOT NULL,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn save_channel(&self, channel: &Channel) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO channels (id, name, display_name, team_id, channel_type)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            [
                &channel.id,
                &channel.name,
                &channel.display_name,
                &channel.team_id,
                &channel.channel_type,
            ],
        )?;
        Ok(())
    }

    pub fn get_channels(&self) -> Result<Vec<Channel>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, display_name, team_id, channel_type FROM channels"
        )?;

        let channels = stmt
            .query_map([], |row| {
                Ok(Channel {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    display_name: row.get(2)?,
                    team_id: row.get(3)?,
                    channel_type: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(channels)
    }

    pub fn save_message(&self, message: &Message) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO messages (id, channel_id, user_id, message, create_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            [
                &message.id,
                &message.channel_id,
                &message.user_id,
                &message.message,
                &message.create_at.to_string(),
            ],
        )?;
        Ok(())
    }

    pub fn get_messages(&self, channel_id: &str, limit: i32) -> Result<Vec<Message>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, channel_id, user_id, message, create_at
             FROM messages
             WHERE channel_id = ?1
             ORDER BY create_at DESC
             LIMIT ?2"
        )?;

        let messages = stmt
            .query_map([channel_id, &limit.to_string()], |row| {
                Ok(Message {
                    id: row.get(0)?,
                    channel_id: row.get(1)?,
                    user_id: row.get(2)?,
                    message: row.get(3)?,
                    create_at: row.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(messages)
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('server_url', ?1)",
            [&settings.server_url],
        )?;

        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('auth_token', ?1)",
            [&settings.auth_token],
        )?;

        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('sync_interval_seconds', ?1)",
            [&settings.sync_interval_seconds.to_string()],
        )?;

        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('max_messages_per_channel', ?1)",
            [&settings.max_messages_per_channel.to_string()],
        )?;

        Ok(())
    }

    pub fn get_settings(&self) -> Result<Settings> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
        let settings_iter = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut settings = Settings::default();

        for setting in settings_iter {
            let (key, value) = setting?;
            match key.as_str() {
                "server_url" => settings.server_url = value,
                "auth_token" => settings.auth_token = value,
                "sync_interval_seconds" => {
                    settings.sync_interval_seconds = value.parse().unwrap_or(5)
                }
                "max_messages_per_channel" => {
                    settings.max_messages_per_channel = value.parse().unwrap_or(100)
                }
                _ => {}
            }
        }

        Ok(settings)
    }

    pub fn queue_message(&self, channel_id: &str, message: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let timestamp = chrono::Utc::now().timestamp_millis();

        conn.execute(
            "INSERT INTO message_queue (channel_id, message, created_at) VALUES (?1, ?2, ?3)",
            [channel_id, message, &timestamp.to_string()],
        )?;
        Ok(())
    }

    pub fn get_queued_messages(&self) -> Result<Vec<(i64, String, String)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, channel_id, message FROM message_queue ORDER BY created_at ASC"
        )?;

        let messages = stmt
            .query_map([], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(messages)
    }

    pub fn remove_queued_message(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM message_queue WHERE id = ?1", [id])?;
        Ok(())
    }
}
