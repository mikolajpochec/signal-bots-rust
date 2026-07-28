use mlua::prelude::*;
use signal_bot_rpc::SignalCliClient;
use tracing::debug;

/// Data passed into each Lua plugin invocation.
/// Wraps the RPC client + message metadata so Lua can call ctx:reply(), etc.
pub struct PluginContext {
    pub client: SignalCliClient,
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
}

impl LuaUserData for PluginContext {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("sender_uuid", |_, this| Ok(this.sender_uuid.clone()));
        fields.add_field_method_get("sender_number", |_, this| Ok(this.sender_number.clone()));
        fields.add_field_method_get("sender_name", |_, this| Ok(this.sender_name.clone()));
        fields.add_field_method_get("group_id", |_, this| Ok(this.group_id.clone()));
        fields.add_field_method_get("text", |_, this| Ok(this.text.clone()));
        fields.add_field_method_get("timestamp", |_, this| Ok(this.timestamp));
        fields.add_field_method_get("is_group", |_, this| Ok(this.is_group));
        fields.add_field_method_get("bot_uptime", |_, this| Ok(this.bot_uptime));
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
            debug!(plugin_reply = %text, "Lua plugin sending reply");
            let client = this.client.clone();
            let is_group = this.is_group;
            let group_id = this.group_id.clone();
            let sender_uuid = this.sender_uuid.clone();

            // Block on the async call from the synchronous Lua context.
            // This is safe because plugin execution runs inside spawn_blocking.
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
    }
}
