# Signal Bot

A modular, extensible Signal messenger bot built in Rust, leveraging the `signal-cli` JSON-RPC interface and powered by a robust `mlua`-based Lua plugin system.

---

## 🏗️ Architecture & File Structure

The project uses a decoupled, multi-crate architecture to separate concerns, making it highly maintainable and easy to extend.

```text
signal-bots/
├── Cargo.toml                  # Workspace definition
├── default-plugins/            # Provided Lua plugins
│   ├── data/                   # Persistent storage for plugins (polls, pins, reminders)
│   ├── dice.lua                # RNG roll plugin
│   ├── lifecycle.lua           # System spawn/death broadcasting
│   ├── pin.lua                 # Pinned messages management
│   ├── poll.lua                # Interactive emoji polls
│   ├── remind.lua              # Persistent scheduled reminders
│   └── wiki.lua                # Wikipedia summaries
├── crates/
│   ├── signal-bot/             # The main CLI entry point & Config parsing (bot.toml)
│   ├── signal-bot-core/        # The Engine, event loop, and lifecycle hooks
│   ├── signal-bot-plugins/     # The mlua PluginManager and Lua API context
│   └── signal-bot-rpc/         # The async interface to the underlying signal-cli daemon
```

### Components
1. **`signal-bot-rpc`**: Handles the underlying Java `signal-cli` child process. It manages Unix sockets, automatically starting the daemon, and parsing inbound/outbound JSON-RPC events.
2. **`signal-bot-core`**: The heart of the bot. The `Engine` sits in a `tokio` event loop reading RPC messages and routing them to the `CommandRouter` (built-in commands) or the `PluginManager`. It handles Graceful Shutdowns on `Ctrl+C`.
3. **`signal-bot-plugins`**: Sandboxes and executes user-defined Lua scripts. Exposes the `PluginContext` API to allow scripts to read/write files, make HTTP requests, send reactions, and schedule persistent tasks.
4. **`signal-bot`**: The binary executable that glues everything together, parses the `bot.toml` configuration, and kicks off the core engine.

---

## ⚙️ Configuration (`bot.toml`)

Before running the bot, you must define a `bot.toml` configuration file. Below is an example structure:

```toml
[bot]
name = "MyAwesomeBot"
prefix = "!"          # The character that triggers commands (e.g. !dice, !wiki)
# profile_picture = "/path/to/image.png" 

[account]
phone = "+1234567890" # The bot's registered Signal phone number
# socket = "/tmp/custom-socket.sock" # Optional, defaults to a generated hash of the phone number

[plugins]
directory = "default-plugins" # Directory containing your .lua scripts

[[groups]]
name = "My Friends Group"
group_id = "group-id-base64-string=" # Group ID where the bot is allowed to respond

[ai]
enabled = true
base_url = "https://openrouter.ai/api/v1"
api_key = "sk-or-v1-..."
model = "google/gemini-2.5-flash-free"
system_prompt = "You are a helpful Signal bot."

[[commands]]
trigger = "ping"
response = "Pong!"
description = "A simple ping command"
```

---

## 🚀 Running the Bot (Spawning and Dying)

Once your configuration is ready, you can compile and spawn the bot using Cargo:

```bash
cargo build --release
cargo run --release -- run --config bot.toml
```

### Spawning (`on_spawn`)
When the engine starts, it automatically spawns the background `signal-cli` Java process, connects via Unix socket, and establishes a JSON-RPC stream. Once fully connected and listening for messages, it fires the **`on_spawn(ctx)`** event to all Lua plugins. 

### Dying (`on_death`)
To gracefully shut down the bot, send a SIGINT (press `Ctrl+C` in your terminal). 
The bot engine intercepts this signal, pauses incoming message processing, and fires the **`on_death(ctx)`** event to all Lua plugins. It waits for the plugins to broadcast their final messages before cleanly terminating the `signal-cli` background daemon.

> **Warning:** Using `SIGKILL` (`kill -9`) will bypass the shutdown sequence and prevent the `on_death` events from firing!

---

## 📜 Lua Scripting Interface

The bot's power lies in its dynamic Lua plugin system. Any `.lua` script placed in the `plugins.directory` is automatically loaded on startup.

### Global Configuration

Plugins can define a few global variables to dictate their behavior:
- `description`: (String) Help text for the `{::prefix}help` menu.
- `aliases`: (Table of Strings) Additional commands that trigger this script.

```lua
description = "Rolls a die. Usage: {::prefix}dice or {::prefix}roll"
aliases = {"roll"}
```

### Event Hooks

Plugins must implement one or more of the following global functions to handle events:

- `function on_command(ctx)`: Invoked when a user types the plugin's trigger or one of its aliases.
- `function on_reaction(ctx)`: Invoked when a user reacts to a message with an emoji.
- `function on_spawn(ctx)`: Invoked when the bot successfully starts up.
- `function on_death(ctx)`: Invoked when the bot is gracefully shutting down (e.g. via `Ctrl+C`).

### The `ctx` (PluginContext) API

The `ctx` object passed to your event hooks provides various fields and methods to interact with Signal and the system.

#### Fields
- `ctx.trigger`: (String) The exact command the user typed (useful for distinguishing aliases).
- `ctx.args`: (Table of Strings) The arguments following the command.
- `ctx.sender_name`, `ctx.sender_number`, `ctx.sender_uuid`: Identity info of the sender.
- `ctx.is_group`, `ctx.group_id`: Group info (if applicable).
- `ctx.bot_uptime`: (Integer) The bot's uptime in seconds.
- `ctx.text`: (String) The raw text of the incoming message.
- *Reaction Fields*: `ctx.reaction_emoji`, `ctx.reaction_target_author`, `ctx.reaction_target_timestamp`, `ctx.reaction_is_remove`.

#### Methods
- **`ctx:reply(text)`**: Sends a standard text reply back to the user or group.
- **`ctx:reply_get_timestamp(text)`**: Sends a reply and returns the sent message's timestamp (useful for reacting to your own messages).
- **`ctx:react_to(target_timestamp, emoji)`**: Sends an emoji reaction to a specific message timestamp.
- **`ctx:send_message(recipient, text)`**: Sends a one-off text message to a specific phone number or UUID.
- **`ctx:send_group_message(group_id, text)`**: Sends a one-off text message to a specific group ID.
- **`ctx:broadcast(text)`**: Sends a message to *all* configured `groups` and `admins` (useful for `on_spawn`/`on_death` announcements).
- **`ctx:schedule_reply(delay_in_seconds, text)`**: Schedules a delayed message. Generates a disk-backed JSON file, meaning the reminder persists even if the bot is restarted! Returns a unique 6-character ID.
- **`ctx:list_reminders()`**: Returns a table of strings formatted as `id|timestamp|text` containing pending reminders for the current context.
- **`ctx:cancel_reminder(id)`**: Cancels a pending reminder by its ID. Returns `true` on success.
- **`ctx:read_file(filename)`**: Reads a text file located in `<plugin_dir>/data/<filename>`.
- **`ctx:write_file(filename, text)`**: Overwrites a text file.
- **`ctx:append_file(filename, text)`**: Appends text to a file.
- **`ctx:http_get(url)`**: Performs a synchronous GET request and returns the response body as a string.
- **`ctx:get_chat_history(limit)`**: Retrieves up to `limit` recent messages from the current chat (scoped to group or DM).
- **`ctx:get_user_history(user_uuid, limit)`**: Retrieves up to `limit` recent messages from a specific user in the current chat context.
- **`ctx:llm_generate(prompt)`**: Sends a prompt to the configured AI API and returns the generated text. (Requires `[ai]` configuration).
- **`ctx:llm_generate_with_context(prompt, history_table)`**: Sends a prompt with an array of context strings to the AI API.

---

## 🛠️ Provided Plugins Overview

The bot comes packaged with a few default plugins demonstrating the power of the API:

- **`lifecycle.lua`**: Uses `on_spawn` and `on_death` to broadcast system boot and shutdown metrics. Users can opt-in to receive these notifications in their chat by using the `!verbose on|off` command, which persists the chat ID using `ctx:write_file`.
- **`poll.lua`**: Listens for `!poll` to create an interactive, multi-option poll. Uses `ctx:reply_get_timestamp()` to immediately attach interactive voting emojis (`ctx:react_to()`). It tracks votes via `on_reaction`.
- **`remind.lua`**: Allows users to set flexible persistent reminders (`!remind 5m Pizza time!`). Implements `ctx:schedule_reply()` and supports aliases (`!reminders`) and deletions (`!reminders rm <id>`).
- **`pin.lua`**: Simple file-backed storage (`ctx:append_file()`) to pin messages (`!pin`). Supports listing and removal (`!pins rm 1`).
- **`wiki.lua`**: Fetches Wikipedia summaries using `ctx:http_get()`. Includes logic to detect and reject disambiguation pages.
- **`dice.lua`**: Standard RNG plugin.
- **`bot.lua` & `roast.lua`**: Demonstrates the `ctx:llm_generate` API to chat with LLMs and seamlessly inject chat history (`ctx:get_chat_history`) into prompts.
