# bots-signal

A modular, extensible Signal messenger bot framework written in Rust.

This framework allows you to easily build bots that read and respond to messages in Signal groups and direct messages. It uses [`signal-cli`](https://github.com/AsamK/signal-cli) as its backend, communicating asynchronously over JSON-RPC.

## Documentation

The full documentation for architecture, configuration (`bot.toml`), and the Lua Plugin API can be found here: 

👉 **[docs/DOCUMENTATION.md](docs/DOCUMENTATION.md)**

## Quickstart

1. **Prerequisites**: You need the Rust toolchain installed (1.75+) and a registered Signal account.
2. **Configure**: Create a `bot.toml` configuration file (see the documentation for details).
3. **Run**: 
   ```bash
   cargo run -- run --config bot.toml
   ```
   > **Note**: The bot will automatically spawn the `signal-cli` daemon in the background for you.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
