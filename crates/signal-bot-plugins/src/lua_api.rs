use mlua::prelude::*;
use signal_bot_rpc::SignalCliClient;
use tracing::debug;

/// Data passed into each Lua plugin invocation.
/// Wraps the RPC client + message metadata so Lua can call ctx:reply(), etc.
#[derive(Clone)]
pub struct PluginContext {
    pub client: SignalCliClient,
    pub prefix: String,
    pub trigger: String,
    pub sender_uuid: String,
    pub sender_number: Option<String>,
    pub sender_name: Option<String>,
    pub group_id: Option<String>,
    pub text: String,
    pub timestamp: u64,
    pub is_group: bool,
    /// Arguments (the text after the command trigger, split by whitespace)
    pub args: Vec<String>,
    /// Bot's uptime in seconds
    pub bot_uptime: u64,
    pub allowed_groups: Vec<String>,
    pub admins: Vec<String>,
    // --- Reaction fields ---
    pub reaction_emoji: Option<String>,
    pub reaction_target_author: Option<String>,
    pub reaction_target_timestamp: Option<u64>,
    pub reaction_is_remove: Option<bool>,
    // --- AI fields ---
    pub ai: Option<AiPluginConfig>,
}

#[derive(Clone, Debug)]
pub struct AiPluginConfig {
    pub enabled: bool,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub system_prompt: String,
}

impl LuaUserData for PluginContext {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("prefix", |_, this| Ok(this.prefix.clone()));
        fields.add_field_method_get("trigger", |_, this| Ok(this.trigger.clone()));
        fields.add_field_method_get("sender_uuid", |_, this| Ok(this.sender_uuid.clone()));
        fields.add_field_method_get("sender_number", |_, this| Ok(this.sender_number.clone()));
        fields.add_field_method_get("sender_name", |_, this| Ok(this.sender_name.clone()));
        fields.add_field_method_get("group_id", |_, this| Ok(this.group_id.clone()));
        fields.add_field_method_get("text", |_, this| Ok(this.text.clone()));
        fields.add_field_method_get("timestamp", |_, this| Ok(this.timestamp));
        fields.add_field_method_get("is_group", |_, this| Ok(this.is_group));
        fields.add_field_method_get("bot_uptime", |_, this| Ok(this.bot_uptime));
        fields.add_field_method_get("reaction_emoji", |_, this| Ok(this.reaction_emoji.clone()));
        fields.add_field_method_get("reaction_target_author", |_, this| Ok(this.reaction_target_author.clone()));
        fields.add_field_method_get("reaction_target_timestamp", |_, this| Ok(this.reaction_target_timestamp));
        fields.add_field_method_get("reaction_is_remove", |_, this| Ok(this.reaction_is_remove));
        fields.add_field_method_get("args", |lua, this| {
            let table = lua.create_table()?;
            for (i, arg) in this.args.iter().enumerate() {
                table.set(i + 1, arg.as_str())?;
            }
            Ok(LuaValue::Table(table))
        });
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // ctx:reply(text) — send a reply to the same conversation
        methods.add_method("reply", |_, this, text: String| {
            let text = text.replace("{::prefix}", &this.prefix);
            debug!(plugin_reply = %text, "Lua plugin sending reply");
            let client = this.client.clone();
            let is_group = this.is_group;
            let group_id = this.group_id.clone();
            let sender_uuid = this.sender_uuid.clone();

            let handle = tokio::runtime::Handle::current();
            handle.block_on(async move {
                if is_group {
                    if let Some(gid) = &group_id {
                        client.send_group_message(gid, &text, &[]).await
                            .map_err(|e| mlua::Error::external(e))?;
                    }
                } else {
                    client.send_message(&sender_uuid, &text, &[]).await
                        .map_err(|e| mlua::Error::external(e))?;
                }
                Ok::<_, mlua::Error>(())
            })?;
            Ok(())
        });

        // ctx:reply_get_timestamp(text) — send a reply and return its timestamp
        methods.add_method("reply_get_timestamp", |_, this, text: String| {
            debug!(plugin_reply = %text, "Lua plugin sending reply with timestamp return");
            let client = this.client.clone();
            let is_group = this.is_group;
            let group_id = this.group_id.clone();
            let sender_uuid = this.sender_uuid.clone();

            let handle = tokio::runtime::Handle::current();
            let ts = handle.block_on(async move {
                let res = if is_group {
                    if let Some(gid) = &group_id {
                        client.send_group_message(gid, &text, &[]).await
                            .map_err(|e| mlua::Error::external(e))?
                    } else {
                        return Err(mlua::Error::external("Missing group ID"));
                    }
                } else {
                    client.send_message(&sender_uuid, &text, &[]).await
                        .map_err(|e| mlua::Error::external(e))?
                };
                Ok::<_, mlua::Error>(res.timestamp)
            })?;
            Ok(ts)
        });

        // ctx:react(emoji) — react to the triggering message
        methods.add_method("react", |_, this, emoji: String| {
            debug!(plugin_react = %emoji, "Lua plugin sending reaction");
            let client = this.client.clone();
            let recipient = if this.is_group {
                this.group_id.clone().unwrap_or_default()
            } else {
                this.sender_uuid.clone()
            };
            let sender_uuid = this.sender_uuid.clone();
            let timestamp = this.timestamp;

            let handle = tokio::runtime::Handle::current();
            handle.block_on(async move {
                client.send_reaction(&recipient, &emoji, &sender_uuid, timestamp).await
                    .map_err(|e| mlua::Error::external(e))?;
                Ok::<_, mlua::Error>(())
            })?;
            Ok(())
        });

        // ctx:react_to(timestamp, emoji) — react to a specific message timestamp
        methods.add_method("react_to", |_, this, (timestamp, emoji): (u64, String)| {
            debug!(plugin_react = %emoji, target_ts = %timestamp, "Lua plugin reacting to specific message");
            let client = this.client.clone();
            let recipient = if this.is_group {
                this.group_id.clone().unwrap_or_default()
            } else {
                this.sender_uuid.clone()
            };
            
            // signal-cli `getUser` is needed to get our own UUID/number for target_author when reacting to our own message.
            // BUT wait, we can just use our own account number! Wait, `this.client` knows what account it's connected to.
            // Actually, in `sendReaction`, if targetAuthor is us, we have to supply our own identifier.
            // A simple hack is to assume the bot's own identifier can be fetched with whoami, or we can just let `signal-cli` default if we provide our own number.
            // For now, let's just use `this.client.whoami()` to get our own identifier!
            
            let handle = tokio::runtime::Handle::current();
            handle.block_on(async move {
                let me = client.whoami().await.map_err(|e| mlua::Error::external(e))?;
                client.send_reaction(&recipient, &emoji, &me, timestamp).await
                    .map_err(|e| mlua::Error::external(e))?;
                Ok::<_, mlua::Error>(())
            })?;
            Ok(())
        });

        // ctx:edit_message(target_timestamp, new_text) — edit a previously sent message, returns new timestamp
        methods.add_method("edit_message", |_, this, (target_timestamp, new_text): (u64, String)| {
            debug!(target_timestamp, "Lua plugin Edit Message");
            let client = this.client.clone();
            let recipient = if !this.is_group { this.sender_number.clone().or_else(|| Some(this.sender_uuid.clone())) } else { None };
            let group_id = if this.is_group { this.group_id.clone() } else { None };
            
            let handle = tokio::runtime::Handle::current();
            let res = handle.block_on(async move {
                client
                    .edit_message(recipient.as_deref(), group_id.as_deref(), &new_text, target_timestamp)
                    .await
                    .map_err(|e| mlua::Error::external(e))
            })?;
            Ok(res.timestamp)
        });

        // ctx:http_get(url) — make a blocking HTTP GET request
        methods.add_method("http_get", |_, _, url: String| {
            debug!(url = %url, "Lua plugin HTTP GET");
            let handle = tokio::runtime::Handle::current();
            let text = handle.block_on(async move {
                let client = reqwest::Client::builder()
                    .user_agent("SignalBot/1.0 (https://github.com/example/signal-bots)")
                    .build()
                    .map_err(|e| mlua::Error::external(e))?;
                client.get(&url).send().await
                    .map_err(|e| mlua::Error::external(e))?
                    .text().await
                    .map_err(|e| mlua::Error::external(e))
            })?;
            Ok(text)
        });

        methods.add_method("get_user_history", |_, this, (user_uuid, limit): (String, u32)| {
            let conn = rusqlite::Connection::open("chat_history.db").map_err(mlua::Error::external)?;
            let mut stmt = if this.is_group {
                let group_id = this.group_id.clone().unwrap_or_default();
                conn.prepare("SELECT text FROM messages WHERE group_id = ?1 AND sender_uuid = ?2 ORDER BY timestamp DESC LIMIT ?3").map_err(mlua::Error::external)?
            } else {
                conn.prepare("SELECT text FROM messages WHERE group_id IS NULL AND sender_uuid = ?1 ORDER BY timestamp DESC LIMIT ?2").map_err(mlua::Error::external)?
            };
            
            let texts: Vec<String> = if this.is_group {
                let group_id = this.group_id.clone().unwrap_or_default();
                let iter = stmt.query_map(rusqlite::params![group_id, user_uuid, limit], |row| row.get(0)).map_err(mlua::Error::external)?;
                iter.filter_map(|r| r.ok()).collect()
            } else {
                let iter = stmt.query_map(rusqlite::params![user_uuid, limit], |row| row.get(0)).map_err(mlua::Error::external)?;
                iter.filter_map(|r| r.ok()).collect()
            };
            Ok(texts)
        });

        methods.add_method("get_chat_history", |_, this, limit: u32| {
            let conn = rusqlite::Connection::open("chat_history.db").map_err(mlua::Error::external)?;
            let mut stmt = if this.is_group {
                let group_id = this.group_id.clone().unwrap_or_default();
                conn.prepare("SELECT text FROM messages WHERE group_id = ?1 ORDER BY timestamp DESC LIMIT ?2").map_err(mlua::Error::external)?
            } else {
                let sender_uuid = this.sender_uuid.clone();
                conn.prepare("SELECT text FROM messages WHERE group_id IS NULL AND sender_uuid = ?1 ORDER BY timestamp DESC LIMIT ?2").map_err(mlua::Error::external)?
            };
            
            let msgs: Vec<String> = if this.is_group {
                let group_id = this.group_id.clone().unwrap_or_default();
                let iter = stmt.query_map(rusqlite::params![group_id, limit], |row| row.get(0)).map_err(mlua::Error::external)?;
                iter.filter_map(|r| r.ok()).collect()
            } else {
                let sender_uuid = this.sender_uuid.clone();
                let iter = stmt.query_map(rusqlite::params![sender_uuid, limit], |row| row.get(0)).map_err(mlua::Error::external)?;
                iter.filter_map(|r| r.ok()).collect()
            };
            Ok(msgs)
        });


        // ctx:append_file(filename, text) — append text to a file in default-plugins/data
        methods.add_method("append_file", |_, _, (filename, text): (String, String)| {
            use std::io::Write;
            if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
                return Err(mlua::Error::external("Invalid filename"));
            }
            let data_dir = std::path::Path::new("default-plugins").join("data");
            std::fs::create_dir_all(&data_dir).map_err(|e| mlua::Error::external(e))?;
            let path = data_dir.join(filename);
            let mut file = std::fs::OpenOptions::new().create(true).append(true).open(path).map_err(|e| mlua::Error::external(e))?;
            file.write_all(text.as_bytes()).map_err(|e| mlua::Error::external(e))?;
            Ok(())
        });

        // ctx:write_file(filename, text) — overwrite a file in default-plugins/data
        methods.add_method("write_file", |_, _, (filename, text): (String, String)| {
            if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
                return Err(mlua::Error::external("Invalid filename"));
            }
            let data_dir = std::path::Path::new("default-plugins").join("data");
            std::fs::create_dir_all(&data_dir).map_err(|e| mlua::Error::external(e))?;
            let path = data_dir.join(filename);
            std::fs::write(path, text).map_err(|e| mlua::Error::external(e))?;
            Ok(())
        });

        // ctx:read_file(filename) — read a file from default-plugins/data
        methods.add_method("read_file", |_, _, filename: String| {
            if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
                return Err(mlua::Error::external("Invalid filename"));
            }
            let path = std::path::Path::new("default-plugins").join("data").join(filename);
            if !path.exists() {
                return Ok(String::new());
            }
            let content = std::fs::read_to_string(path).map_err(|e| mlua::Error::external(e))?;
            Ok(content)
        });

        // ctx:schedule_reply(delay_seconds, text) — wait and then reply
        methods.add_method("schedule_reply", |_, this, (delay, text): (u64, String)| {
            debug!(delay = %delay, "Lua plugin scheduling reply");
            let client = this.client.clone();
            let is_group = this.is_group;
            let group_id = this.group_id.clone();
            let sender_uuid = this.sender_uuid.clone();

            let target_time = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() + delay;
            let reminder = crate::manager::ScheduledReminder {
                target_time,
                is_group,
                group_id: group_id.clone(),
                sender_uuid: sender_uuid.clone(),
                text: text.clone(),
            };

            let reminders_dir = std::path::Path::new("default-plugins").join("data").join("reminders");
            let _ = std::fs::create_dir_all(&reminders_dir);
            
            // Generate a simple sequential ID (1 to infinity)
            let mut next_id = 1;
            if let Ok(entries) = std::fs::read_dir(&reminders_dir) {
                for entry in entries.filter_map(Result::ok) {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.ends_with(".json") {
                            if let Ok(num) = name.trim_end_matches(".json").parse::<u64>() {
                                if num >= next_id {
                                    next_id = num + 1;
                                }
                            }
                        }
                    }
                }
            }
            
            let id = next_id.to_string();
            let path = reminders_dir.join(format!("{}.json", id));

            if let Ok(json) = serde_json::to_string(&reminder) {
                let _ = std::fs::write(&path, json);
            }

            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                if path.exists() {
                    if is_group {
                        if let Some(gid) = &group_id {
                            let _ = client.send_group_message(gid, &text, &[]).await;
                        }
                    } else {
                        let _ = client.send_message(&sender_uuid, &text, &[]).await;
                    }
                    let _ = std::fs::remove_file(path);
                }
            });
            Ok(id)
        });

        // ctx:list_reminders() — list all scheduled reminders for this context (sender/group)
        methods.add_method("list_reminders", |_, this, ()| {
            let mut results = Vec::new();
            let reminders_dir = std::path::Path::new("default-plugins").join("data").join("reminders");
            if let Ok(entries) = std::fs::read_dir(&reminders_dir) {
                for entry in entries.filter_map(Result::ok) {
                    let path = entry.path();
                    if path.extension().map(|s| s == "json").unwrap_or(false) {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            if let Ok(reminder) = serde_json::from_str::<crate::manager::ScheduledReminder>(&content) {
                                // Filter by current context
                                let is_match = if this.is_group {
                                    reminder.is_group && reminder.group_id == this.group_id
                                } else {
                                    !reminder.is_group && reminder.sender_uuid == this.sender_uuid
                                };
                                
                                if is_match {
                                    if let Some(id) = path.file_stem().and_then(|s| s.to_str()) {
                                        results.push(format!("{}|{}|{}", id, reminder.target_time, reminder.text));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Ok(results)
        });

        // ctx:cancel_reminder(id) — cancel a reminder by its ID
        methods.add_method("cancel_reminder", |_, _, id: String| {
            if id.contains('/') || id.contains('\\') || id.contains("..") {
                return Err(mlua::Error::external("Invalid ID"));
            }
            let path = std::path::Path::new("default-plugins").join("data").join("reminders").join(format!("{}.json", id));
            if path.exists() {
                let _ = std::fs::remove_file(path);
                Ok(true)
            } else {
                Ok(false)
            }
        });

        // ctx:broadcast(text) — send a message to all allowed groups and admins
        methods.add_method("broadcast", |_, this, text: String| {
            debug!(groups = ?this.allowed_groups, admins = ?this.admins, "Lua plugin broadcasting message");
            let client = this.client.clone();
            let groups = this.allowed_groups.clone();
            let admins = this.admins.clone();
            
            tokio::spawn(async move {
                for gid in groups {
                    let _ = client.send_group_message(&gid, &text, &[]).await;
                }
                for admin in admins {
                    let _ = client.send_message(&admin, &text, &[]).await;
                }
            });
            Ok(())
        });

        // ctx:send_message(recipient, text) — send a message to a specific number/uuid
        methods.add_method("send_message", |_, this, (recipient, text): (String, String)| {
            let text = text.replace("{::prefix}", &this.prefix);
            let client = this.client.clone();
            tokio::spawn(async move {
                if let Err(e) = client.send_message(&recipient, &text, &[]).await {
                    tracing::error!("Failed to send_message to {}: {}", recipient, e);
                }
            });
            Ok(())
        });

        // ctx:send_group_message(group_id, text) — send a message to a specific group
        methods.add_method("send_group_message", |_, this, (group_id, text): (String, String)| {
            let text = text.replace("{::prefix}", &this.prefix);
            let client = this.client.clone();
            tokio::spawn(async move {
                if let Err(e) = client.send_group_message(&group_id, &text, &[]).await {
                    tracing::error!("Failed to send_group_message to {}: {}", group_id, e);
                }
            });
            Ok(())
        });
        // ctx:llm_generate(prompt)
        methods.add_method("llm_generate", |_, this, prompt: String| {
            let ai_cfg = this.ai.clone().ok_or_else(|| mlua::Error::external("AI is not configured"))?;
            if !ai_cfg.enabled {
                return Err(mlua::Error::external("AI is disabled"));
            }

            let handle = tokio::runtime::Handle::current();
            let response = handle.block_on(async move {
                let url = format!("{}/chat/completions", ai_cfg.base_url.trim_end_matches('/'));
                let mut messages = Vec::new();
                if !ai_cfg.system_prompt.is_empty() {
                    messages.push(serde_json::json!({"role": "system", "content": ai_cfg.system_prompt}));
                }
                messages.push(serde_json::json!({"role": "user", "content": prompt}));

                let body = serde_json::json!({
                    "model": ai_cfg.model,
                    "messages": messages
                });

                let client = reqwest::Client::new();
                let res = client.post(&url)
                    .bearer_auth(&ai_cfg.api_key)
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| mlua::Error::external(e))?;
                
                if !res.status().is_success() {
                    let status = res.status();
                    let text = res.text().await.unwrap_or_default();
                    return Err(mlua::Error::external(format!("AI error {}: {}", status, text)));
                }

                let parsed: serde_json::Value = res.json().await.map_err(|e| mlua::Error::external(e))?;
                let content = parsed.get("choices")
                    .and_then(|c| c.as_array())
                    .and_then(|c| c.first())
                    .and_then(|c| c.get("message"))
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string();

                Ok::<_, mlua::Error>(content)
            })?;
            Ok(response)
        });

        // ctx:llm_generate_with_context(prompt, context_messages)
        methods.add_method("llm_generate_with_context", |_, this, (prompt, context_messages): (String, Vec<String>)| {
            let ai_cfg = this.ai.clone().ok_or_else(|| mlua::Error::external("AI is not configured"))?;
            if !ai_cfg.enabled {
                return Err(mlua::Error::external("AI is disabled"));
            }

            let handle = tokio::runtime::Handle::current();
            let response = handle.block_on(async move {
                let url = format!("{}/chat/completions", ai_cfg.base_url.trim_end_matches('/'));
                let mut messages = Vec::new();
                if !ai_cfg.system_prompt.is_empty() {
                    messages.push(serde_json::json!({"role": "system", "content": ai_cfg.system_prompt}));
                }
                for msg in context_messages {
                    messages.push(serde_json::json!({"role": "user", "content": msg}));
                }
                messages.push(serde_json::json!({"role": "user", "content": prompt}));

                let body = serde_json::json!({
                    "model": ai_cfg.model,
                    "messages": messages
                });

                let client = reqwest::Client::new();
                let res = client.post(&url)
                    .bearer_auth(&ai_cfg.api_key)
                    .json(&body)
                    .send()
                    .await
                    .map_err(|e| mlua::Error::external(e))?;
                
                if !res.status().is_success() {
                    let status = res.status();
                    let text = res.text().await.unwrap_or_default();
                    return Err(mlua::Error::external(format!("AI error {}: {}", status, text)));
                }

                let parsed: serde_json::Value = res.json().await.map_err(|e| mlua::Error::external(e))?;
                let content = parsed.get("choices")
                    .and_then(|c| c.as_array())
                    .and_then(|c| c.first())
                    .and_then(|c| c.get("message"))
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string();

                Ok::<_, mlua::Error>(content)
            })?;
            Ok(response)
        });
    }
}
