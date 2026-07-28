//! Signal Bot Plugin System
//!
//! Provides Lua-based scripting support for signal-bot.
//! Plugins are `.lua` files loaded from a directory.

pub mod error;
pub mod manager;
pub mod lua_api;

pub use error::PluginError;
pub use manager::PluginManager;
