use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct BotConfig {
    pub bot: BotSection,
    pub account: AccountSection,
    #[serde(default)]
    pub groups: Vec<GroupConfig>,
    #[serde(default)]
    pub commands: Vec<CommandConfig>,
    pub webhooks: Option<WebhooksConfig>,
    pub rate_limit: Option<RateLimitConfig>,
    pub plugins: Option<PluginsConfig>,
}

#[derive(Debug, Deserialize)]
pub struct BotSection {
    pub name: String,
    #[serde(default = "default_prefix")]
    pub prefix: String,
    pub profile_picture: Option<String>,
    #[serde(default)]
    pub admins: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct AccountSection {
    /// Path to signal-cli socket (Unix path or tcp://host:port). If omitted, generated from phone number.
    pub socket: Option<String>,
    /// Phone number of the bot account (e.g. +1234567890)
    pub phone: String,
}

impl AccountSection {
    pub fn effective_socket(&self) -> String {
        self.socket.clone().unwrap_or_else(|| {
            use std::hash::{Hash, Hasher};
            use std::collections::hash_map::DefaultHasher;
            let mut hasher = DefaultHasher::new();
            self.phone.hash(&mut hasher);
            let hash = hasher.finish();
            format!("/tmp/signal-cli-{:x}.sock", hash)
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct GroupConfig {
    pub name: String,
    pub group_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CommandConfig {
    pub trigger: String,
    pub response: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct WebhooksConfig {
    pub incoming_url: Option<String>,
    pub outgoing_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RateLimitConfig {
    #[serde(default = "default_messages_per_minute")]
    pub messages_per_minute: u32,
    #[serde(default = "default_cooldown_seconds")]
    pub cooldown_seconds: u64,
}

#[derive(Debug, Deserialize)]
pub struct PluginsConfig {
    /// Path to the directory containing .lua plugin files
    pub directory: String,
}

fn default_prefix() -> String { "!".into() }
fn default_messages_per_minute() -> u32 { 20 }
fn default_cooldown_seconds() -> u64 { 1 }

impl BotConfig {
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: BotConfig = toml::from_str(&content)?;
        Ok(config)
    }
}
