# bots-signal

A (mostly) vibe-coded modular, high-performance Signal Messenger bot framework written in Rust.

This framework allows you to easily build bots that read and respond to messages in Signal groups and direct messages. It uses [`signal-cli`](https://github.com/AsamK/signal-cli) as its backend, communicating asynchronously over JSON-RPC.

## Architecture

To ensure stability and keep the project lightweight, `bots-signal` acts as an RPC client to a running `signal-cli` daemon.

- **signal-bot**: The CLI application that parses the `bot.toml` configuration and runs the engine.
- **signal-bot-core**: The central bot engine, handling rate limiting, context injection, and command routing.
- **signal-bot-rpc**: An async JSON-RPC client for `signal-cli`, handling both Unix domain sockets and TCP connections.
- **signal-bot-plugins**: A built-in Lua 5.4 engine using `mlua` that loads `.lua` scripts dynamically as bot commands without recompiling the Rust core.

## Prerequisites

1. **Rust**: You need the Rust toolchain installed (1.75+).
2. **signal-cli**: You must have `signal-cli` installed and registered with a Signal account.
   - [signal-cli Installation Guide](https://github.com/AsamK/signal-cli#installation)
   - You must link or register a device first before the bot can use it.

## Quickstart

### 1. Start `signal-cli` in Daemon Mode

The bot requires `signal-cli` to be running in JSON-RPC daemon mode. You can easily start it using the built-in bot command (after configuring your `bot.toml` in step 2):

```bash
cargo run -- daemon --config bot.toml
```

### 2. Configure the Bot

Create a `bot.toml` configuration file. See `examples/echo-bot/bot.toml` for an example.

```toml
[bot]
name = "MyAwesomeBot"
prefix = "!"
log_level = "info"

[account]
# The socket path must match the one used by signal-cli
socket = "/tmp/signal-cli.sock"
# Or for TCP: socket = "tcp://127.0.0.1:7583"
# The phone number of your bot
phone = "+1234567890"

[plugins]
directory = "./plugins"

[[commands]]
trigger = "ping"
response = "pong"
description = "Check if the bot is alive"
```

### 3. Write a Lua Plugin (Optional)

Create a `./plugins/dice.lua` file. The filename becomes the command trigger (e.g. `!dice`).

```lua
description = "Roll a dice (e.g. !dice 6)"

function on_command(ctx)
    local sides = tonumber(ctx.args[1]) or 6
    if sides < 1 then sides = 6 end
    local result = math.random(1, sides)
    ctx:reply("🎲 You rolled a " .. tostring(result) .. " (d" .. tostring(sides) .. ")")
end
```

Available context methods in Lua:
- `ctx:reply(text)` — send a message to the same conversation
- `ctx:react(emoji)` — react to the triggering message
- Context fields: `ctx.sender_uuid`, `ctx.sender_name`, `ctx.group_id`, `ctx.text`, `ctx.args`

### 4. Build and Run

Compile the project:

```bash
cargo build --release
```

Run the bot:

```bash
cargo run -- run --config bot.toml -v
```

> **Auto-Registration:** If your bot's phone number isn't registered with Signal yet, running the bot will automatically pause and guide you through an interactive registration wizard in the terminal, including handling CAPTCHAs and SMS verification codes.

## CLI Usage

The `signal-bot` binary provides several utilities:

- **Start signal-cli daemon**:
  `signal-bot daemon --config path/to/bot.toml` (Spawns `signal-cli` directly based on config)
- **Run the bot engine**:
  `signal-bot run --config path/to/bot.toml`
- **Send a one-off message**:
  `signal-bot send --socket /tmp/signal-cli.sock --recipient +1234567890 --message "Hello from CLI"`
- **List available groups**:
  `signal-bot groups --socket /tmp/signal-cli.sock list`

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
