use std::collections::HashMap;
use std::path::{Path, PathBuf};

use mlua::prelude::*;
use tracing::{info, warn};

use crate::error::PluginError;
use crate::lua_api::PluginContext;

/// Metadata about a loaded plugin.
pub struct PluginInfo {
    /// The command trigger (derived from filename, e.g. "dice" from "dice.lua")
    pub trigger: String,
    /// Human-readable description (from plugin's `description` global, or empty)
    pub description: String,
    /// The Lua source code (kept for reload)
    source: String,
}

/// Manages loading and executing Lua plugins.
pub struct PluginManager {
    /// Directory where .lua plugin files live
    plugin_dir: PathBuf,
    /// Map of trigger name → PluginInfo
    plugins: HashMap<String, PluginInfo>,
}

impl PluginManager {
    /// Create a new PluginManager. Does NOT load plugins yet.
    pub fn new(plugin_dir: impl Into<PathBuf>) -> Self {
        Self {
            plugin_dir: plugin_dir.into(),
            plugins: HashMap::new(),
        }
    }

    /// Load (or reload) all `.lua` files from the plugin directory.
    /// Each file must define a global function `on_command(ctx)`.
    /// Optionally, it can set a global string `description`.
    ///
    /// The trigger name is the filename stem (e.g. `dice.lua` → trigger `dice`).
    pub fn load_all(&mut self) -> Result<usize, PluginError> {
        if !self.plugin_dir.exists() {
            return Err(PluginError::DirectoryNotFound(
                self.plugin_dir.display().to_string(),
            ));
        }

        self.plugins.clear();
        let mut count = 0;

        let entries = std::fs::read_dir(&self.plugin_dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("lua") {
                continue;
            }

            let trigger = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();

            match self.load_plugin(&path, &trigger) {
                Ok(info) => {
                    info!(trigger = %info.trigger, desc = %info.description, "Loaded plugin");
                    self.plugins.insert(trigger, info);
                    count += 1;
                }
                Err(e) => {
                    warn!("Failed to load plugin {}: {}", path.display(), e);
                }
            }
        }

        info!("Loaded {} plugin(s) from {}", count, self.plugin_dir.display());
        Ok(count)
    }

    /// Load a single plugin file. Validates that it defines `on_command`.
    fn load_plugin(&self, path: &Path, trigger: &str) -> Result<PluginInfo, PluginError> {
        let source = std::fs::read_to_string(path)?;

        // Create a temporary Lua state to validate the plugin
        let lua = Lua::new();
        lua.load(&source).exec().map_err(|e| PluginError::LoadError {
            name: trigger.to_string(),
            source: e,
        })?;

        // Check that on_command exists
        let globals = lua.globals();
        let _handler: LuaFunction = globals.get("on_command").map_err(|_| {
            PluginError::MissingHandler(trigger.to_string())
        })?;

        // Read optional description
        let description: String = globals
            .get::<String>("description")
            .unwrap_or_else(|_| format!("Lua plugin: {}", trigger));

        Ok(PluginInfo {
            trigger: trigger.to_string(),
            description,
            source,
        })
    }

    /// Execute a plugin for the given trigger.
    /// Returns `None` if no plugin matches the trigger.
    /// Uses spawn_blocking because Lua execution is synchronous.
    pub async fn execute(
        &self,
        trigger: &str,
        plugin_ctx: PluginContext,
    ) -> Option<Result<(), PluginError>> {
        let info = self.plugins.get(trigger)?;
        let source = info.source.clone();
        let trigger_owned = trigger.to_string();

        let result = tokio::task::spawn_blocking(move || {
            let lua = Lua::new();

            // Load and execute the plugin source to define on_command
            lua.load(&source).exec().map_err(|e| PluginError::ExecutionError {
                name: trigger_owned.clone(),
                source: e,
            })?;

            // Set the context as a global
            lua.globals()
                .set("ctx", plugin_ctx)
                .map_err(|e| PluginError::ExecutionError {
                    name: trigger_owned.clone(),
                    source: e,
                })?;

            // Call on_command(ctx)
            let on_command: LuaFunction = lua.globals().get("on_command")
                .map_err(|e| PluginError::ExecutionError {
                    name: trigger_owned.clone(),
                    source: e,
                })?;

            let ctx_val: LuaValue = lua.globals().get("ctx")
                .map_err(|e| PluginError::ExecutionError {
                    name: trigger_owned.clone(),
                    source: e,
                })?;

            on_command.call::<()>(ctx_val).map_err(|e| PluginError::ExecutionError {
                name: trigger_owned.clone(),
                source: e,
            })?;

            Ok(())
        })
        .await
        .unwrap_or_else(|e| Err(PluginError::ExecutionError {
            name: trigger.to_string(),
            source: mlua::Error::external(e),
        }));

        Some(result)
    }

    /// Broadcast a reaction context to all loaded plugins that define `on_reaction`.
    pub async fn broadcast_reaction(&self, plugin_ctx: PluginContext) {
        let mut tasks = Vec::new();
        for (trigger, info) in &self.plugins {
            let source = info.source.clone();
            let trigger_owned = trigger.clone();
            let ctx = plugin_ctx.clone();

            tasks.push(tokio::task::spawn_blocking(move || {
                let lua = Lua::new();

                if let Err(_) = lua.load(&source).exec() {
                    return;
                }

                if let Ok(on_reaction) = lua.globals().get::<LuaFunction>("on_reaction") {
                    if let Ok(_) = lua.globals().set("ctx", ctx) {
                        if let Ok(ctx_val) = lua.globals().get::<LuaValue>("ctx") {
                            if let Err(e) = on_reaction.call::<()>(ctx_val) {
                                tracing::error!("Error executing on_reaction in plugin {}: {}", trigger_owned, e);
                            }
                        }
                    }
                }
            }));
        }

        for task in tasks {
            let _ = task.await;
        }
    }

    /// Returns an iterator over all loaded plugin triggers and descriptions.
    pub fn list(&self) -> impl Iterator<Item = (&str, &str)> {
        self.plugins
            .values()
            .map(|info| (info.trigger.as_str(), info.description.as_str()))
    }

    /// Check if a trigger matches a loaded plugin.
    pub fn has_plugin(&self, trigger: &str) -> bool {
        self.plugins.contains_key(trigger)
    }

    /// Get the plugin directory path.
    pub fn plugin_dir(&self) -> &Path {
        &self.plugin_dir
    }
}
