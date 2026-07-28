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
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

#[derive(Debug, Deserialize)]
pub struct AccountSection {
    /// Path to signal-cli socket (Unix path or tcp://host:port)
    pub socket: String,
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
fn default_log_level() -> String { "info".into() }
fn default_messages_per_minute() -> u32 { 20 }
fn default_cooldown_seconds() -> u64 { 1 }

impl BotConfig {
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: BotConfig = toml::from_str(&content)?;
        Ok(config)
    }
}
