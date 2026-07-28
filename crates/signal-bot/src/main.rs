mod cli;
mod config;
mod logging;

use clap::Parser;

// We assume these types will be provided by the parallel crates
// signal_bot_rpc::client::SignalCliClient
// signal_bot_core::engine::Engine
// signal_bot_core::commands::{CommandRouter, Command}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = cli::Cli::parse();
    logging::init(args.verbose);

    match args.command {
        cli::Command::Run { config } => {
            let config_data = config::BotConfig::load(&config)?;
            
            // Note: Assuming SignalCliClient::connect handles both tcp:// and unix paths
            // or the parallel agent implements it this way.
            let client = if config_data.account.socket.starts_with("tcp://") {
                signal_bot_rpc::client::SignalCliClient::connect_tcp(
                    config_data.account.socket.strip_prefix("tcp://").unwrap()
                ).await?
            } else {
                signal_bot_rpc::client::SignalCliClient::connect_unix(&config_data.account.socket).await?
            };

            let mut router = signal_bot_core::commands::CommandRouter::new(&config_data.bot.prefix);
            
            for cmd in config_data.commands {
                // Assuming a method like register_static or similar exists, or a simple add_command
                router.register_static(&cmd.trigger, &cmd.response, &cmd.description);
            }

            // Load plugins if configured
            let plugin_manager = if let Some(plugins_cfg) = &config_data.plugins {
                let mut pm = signal_bot_plugins::PluginManager::new(&plugins_cfg.directory);
                match pm.load_all() {
                    Ok(count) => {
                        tracing::info!("Loaded {} Lua plugin(s)", count);
                        Some(pm)
                    }
                    Err(e) => {
                        tracing::warn!("Failed to load plugins: {}", e);
                        None
                    }
                }
            } else {
                None
            };

            let engine = signal_bot_core::engine::Engine::new(
                client, router, None, vec![], plugin_manager,
            );
            engine.run().await?;
        },
        cli::Command::Send { recipient, group, message, socket } => {
            let client = if socket.starts_with("tcp://") {
                signal_bot_rpc::client::SignalCliClient::connect_tcp(socket.strip_prefix("tcp://").unwrap()).await?
            } else {
                signal_bot_rpc::client::SignalCliClient::connect_unix(&socket).await?
            };

            if let Some(r) = recipient {
                client.send_message(&r, &message, &[]).await?;
            } else if let Some(g) = group {
                client.send_group_message(&g, &message, &[]).await?;
            } else {
                anyhow::bail!("Must specify recipient or group");
            }
            println!("Message sent successfully.");
        },
        cli::Command::Groups { socket, subcommand } => {
            let client = if socket.starts_with("tcp://") {
                signal_bot_rpc::client::SignalCliClient::connect_tcp(socket.strip_prefix("tcp://").unwrap()).await?
            } else {
                signal_bot_rpc::client::SignalCliClient::connect_unix(&socket).await?
            };

            match subcommand {
                cli::GroupsCommand::List => {
                    let groups = client.list_groups().await?;
                    println!("{:<30} | {}", "Group ID", "Name");
                    println!("{:-<30}-|-{:-<30}", "", "");
                    for g in groups {
                        // Assuming Group struct has id and name fields
                        println!("{:<30} | {}", g.id, g.name.unwrap_or_default());
                    }
                }
            }
        },
        cli::Command::Version => {
            println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        },
    }

    Ok(())
}
