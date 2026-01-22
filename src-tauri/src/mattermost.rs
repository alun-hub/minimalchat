use crate::models::{Channel, Message};
use reqwest::{Client, header};
use serde_json::json;
use std::error::Error;

pub struct MattermostClient {
    base_url: String,
    token: String,
    client: Client,
}

impl MattermostClient {
    pub fn new(base_url: String, token: String) -> Result<Self, Box<dyn Error>> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {}", token))?,
        );
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );

        let client = Client::builder()
            .default_headers(headers)
            .gzip(true) // Enable gzip compression to reduce bandwidth
            .build()?;

        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
            client,
        })
    }

    pub async fn get_user_info(&self) -> Result<serde_json::Value, Box<dyn Error>> {
        let url = format!("{}/api/v4/users/me", self.base_url);
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()).into());
        }

        Ok(response.json().await?)
    }

    pub async fn get_channels(&self) -> Result<Vec<Channel>, Box<dyn Error>> {
        // First get user info to get team ID
        let user_info = self.get_user_info().await?;
        let user_id = user_info["id"].as_str()
            .ok_or("No user ID found")?;

        // Get teams
        let teams_url = format!("{}/api/v4/users/{}/teams", self.base_url, user_id);
        let teams_response = self.client.get(&teams_url).send().await?;
        let teams: Vec<serde_json::Value> = teams_response.json().await?;

        if teams.is_empty() {
            return Ok(Vec::new());
        }

        let team_id = teams[0]["id"].as_str()
            .ok_or("No team ID found")?;

        // Get channels for the team
        let channels_url = format!(
            "{}/api/v4/users/{}/teams/{}/channels",
            self.base_url, user_id, team_id
        );
        let response = self.client.get(&channels_url).send().await?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()).into());
        }

        Ok(response.json().await?)
    }

    pub async fn get_channel_messages(
        &self,
        channel_id: &str,
        limit: i32,
    ) -> Result<Vec<Message>, Box<dyn Error>> {
        let url = format!(
            "{}/api/v4/channels/{}/posts?per_page={}",
            self.base_url, channel_id, limit
        );
        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()).into());
        }

        let data: serde_json::Value = response.json().await?;
        let mut messages = Vec::new();

        if let Some(posts) = data["posts"].as_object() {
            for (_, post) in posts {
                if let (Some(id), Some(channel_id), Some(user_id), Some(message), Some(create_at)) = (
                    post["id"].as_str(),
                    post["channel_id"].as_str(),
                    post["user_id"].as_str(),
                    post["message"].as_str(),
                    post["create_at"].as_i64(),
                ) {
                    messages.push(Message {
                        id: id.to_string(),
                        channel_id: channel_id.to_string(),
                        user_id: user_id.to_string(),
                        message: message.to_string(),
                        create_at,
                    });
                }
            }
        }

        // Sort by timestamp (newest first)
        messages.sort_by(|a, b| b.create_at.cmp(&a.create_at));

        Ok(messages)
    }

    pub async fn send_message(
        &self,
        channel_id: &str,
        message: &str,
    ) -> Result<(), Box<dyn Error>> {
        let url = format!("{}/api/v4/posts", self.base_url);
        let body = json!({
            "channel_id": channel_id,
            "message": message,
        });

        let response = self.client.post(&url).json(&body).send().await?;

        if !response.status().is_success() {
            return Err(format!("HTTP error: {}", response.status()).into());
        }

        Ok(())
    }
}
