# bots-signal

A vibe-coded modular, high-performance Signal Messenger bot framework written in Rust.

This framework allows you to easily build bots that read and respond to messages in Signal groups and direct messages. It uses [`signal-cli`](https://github.com/AsamK/signal-cli) as its backend, communicating asynchronously over JSON-RPC.

## Architecture

To ensure stability and keep the project lightweight, `bots-signal` acts as an RPC client to a running `signal-cli` daemon.

- **signal-bot**: The CLI application that parses the `bot.toml` configuration and runs the engine.
- **signal-bot-core**: The central bot engine, handling rate limiting, context injection, and command routing.
- **signal-bot-rpc**: An async JSON-RPC client for `signal-cli`, handling both Unix domain sockets and TCP connections.
- **signal-bot-plugins**: (Upcoming) A plugin system for loading dynamic bot behaviors.

## Prerequisites

1. **Rust**: You need the Rust toolchain installed (1.75+).
2. **signal-cli**: You must have `signal-cli` installed and registered with a Signal account.
   - [signal-cli Installation Guide](https://github.com/AsamK/signal-cli#installation)
   - You must link or register a device first before the bot can use it.

## Quickstart

### 1. Start `signal-cli` in Daemon Mode

The bot requires `signal-cli` to be running in JSON-RPC daemon mode.

```bash
# Using a Unix socket (recommended)
signal-cli -u YOUR_NUMBER daemon --socket /tmp/signal-cli.sock
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

[[commands]]
trigger = "ping"
response = "pong"
description = "Check if the bot is alive"
```

### 3. Build and Run

Compile the project:

```bash
cargo build --release
```

Run the bot:

```bash
cargo run -- run --config bot.toml -v
```

The bot will now listen for incoming messages starting with `!ping` and automatically respond.

## CLI Usage

The `signal-bot` binary provides several utilities:

- **Run the bot engine**:
  `signal-bot run --config path/to/bot.toml`
- **Send a one-off message**:
  `signal-bot send --socket /tmp/signal-cli.sock --recipient +1234567890 --message "Hello from CLI"`
- **List available groups**:
  `signal-bot groups --socket /tmp/signal-cli.sock list`

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
