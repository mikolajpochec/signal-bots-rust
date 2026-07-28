use thiserror::Error;

#[derive(Error, Debug)]
pub enum BotError {
    #[error("RPC error: {0}")]
    Rpc(#[from] signal_bot_rpc::RpcError),
    #[error("Command not found: {0}")]
    CommandNotFound(String),
    #[error("Plugin error: {0}")]
    PluginError(String),
    #[error("Config error: {0}")]
    ConfigError(String),
    #[error("Rate limited")]
    RateLimited,
}
