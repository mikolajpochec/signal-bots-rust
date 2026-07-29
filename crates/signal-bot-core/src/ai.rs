use serde::{Deserialize, Serialize};
use reqwest::Client;
use crate::error::BotError;

#[derive(Debug, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, Deserialize)]
pub struct ChatChoice {
    pub message: ChatMessageResponse,
}

#[derive(Debug, Deserialize)]
pub struct ChatMessageResponse {
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    pub choices: Vec<ChatChoice>,
}

pub struct AiClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl AiClient {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            api_key,
        }
    }

    pub async fn generate(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
    ) -> Result<String, BotError> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let req = ChatRequest {
            model: model.to_string(),
            messages,
        };

        let res = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&req)
            .send()
            .await
            .map_err(|e| BotError::Internal(e.to_string()))?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(BotError::Internal(format!("AI API error: {} - {}", status, body)));
        }

        let mut parsed: ChatResponse = res
            .json()
            .await
            .map_err(|e| BotError::Internal(e.to_string()))?;

        if parsed.choices.is_empty() {
            return Err(BotError::Internal("No choices returned from AI API".to_string()));
        }

        Ok(parsed.choices.remove(0).message.content)
    }
}
