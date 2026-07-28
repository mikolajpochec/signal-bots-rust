use thiserror::Error;

#[derive(Error, Debug)]
pub enum PluginError {
    #[error("Failed to load plugin '{name}': {source}")]
    LoadError {
        name: String,
        source: mlua::Error,
    },

    #[error("Plugin '{name}' execution failed: {source}")]
    ExecutionError {
        name: String,
        source: mlua::Error,
    },

    #[error("Plugin directory not found: {0}")]
    DirectoryNotFound(String),

    #[error("Plugin '{0}' missing required 'on_command' function")]
    MissingHandler(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}
