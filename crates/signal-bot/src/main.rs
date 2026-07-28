mod cli;
mod config;
mod logging;

use clap::Parser;

// We assume these types will be provided by the parallel crates
// signal_bot_rpc::client::SignalCliClient
// signal_bot_core::engine::Engine
// signal_bot_core::commands::{CommandRouter, Command}

async fn interactive_registration(client: &signal_bot_rpc::client::SignalCliClient, phone: &str) -> anyhow::Result<()> {
    use std::io::Write;
    
    println!("⚠️  Bot is not registered with Signal yet.");
    println!("Attempting to register account {}...", phone);
    
    // First try without captcha
    match client.register(phone, None, false).await {
        Ok(_) => {
            println!("✅ Registration request sent successfully.");
        }
        Err(e) => {
            let err_str = e.to_string().to_lowercase();
            if err_str.contains("captcha") {
                println!("🛡️  Signal requires a CAPTCHA token to register.");
                println!("1. Go to: https://signalcaptchas.org/registration/generate.html");
                println!("2. Solve the captcha.");
                println!("3. Copy the 'signalcaptcha://...' link or token.");
                print!("Paste the token here: ");
                std::io::stdout().flush()?;
                
                let mut captcha = String::new();
                std::io::stdin().read_line(&mut captcha)?;
                let captcha = captcha.trim();
                
                println!("Retrying registration with CAPTCHA...");
                client.register(phone, Some(captcha), false).await?;
                println!("✅ Registration request sent successfully.");
            } else {
                anyhow::bail!("Failed to register: {}", e);
            }
        }
    }

    print!("Enter the SMS verification code you received: ");
    std::io::stdout().flush()?;
    
    let mut code = String::new();
    std::io::stdin().read_line(&mut code)?;
    let code = code.trim().replace("-", ""); // Strip dashes if any

    println!("Verifying code...");
    client.verify(phone, &code, None).await?;
    
    println!("🎉 Successfully registered and verified!");
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = cli::Cli::parse();
    logging::init(args.verbose);

    match args.command {
        cli::Command::Run { config } => {
            let config_data = config::BotConfig::load(&config)?;
            let effective_socket = config_data.account.effective_socket();
            let phone = config_data.account.phone.clone();
            
            tracing::info!("Starting signal-cli daemon for {} on {}", phone, effective_socket);
            let mut cmd = tokio::process::Command::new("signal-cli");
            cmd.arg("-u").arg(&phone).arg("daemon").arg("--socket").arg(&effective_socket);
            cmd.kill_on_drop(true); // Automatically kill the daemon when the bot exits
            
            let _daemon_child = cmd.spawn()?;
            
            // Give the daemon a moment to create the socket/bind to port
            let mut retries = 0;
            let client = loop {
                let res = if effective_socket.starts_with("tcp://") {
                    signal_bot_rpc::client::SignalCliClient::connect_tcp(
                        effective_socket.strip_prefix("tcp://").unwrap()
                    ).await
                } else {
                    signal_bot_rpc::client::SignalCliClient::connect_unix(&effective_socket).await
                };
                
                match res {
                    Ok(c) => break c,
                    Err(e) => {
                        if retries >= 15 {
                            anyhow::bail!("Failed to connect to signal-cli daemon: {}", e);
                        }
                        retries += 1;
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    }
                }
            };

            // Auto-registration check
            if let Err(e) = client.whoami().await {
                tracing::warn!("Failed to identify account (might not be registered): {}", e);
                interactive_registration(&client, &config_data.account.phone).await?;
            } else {
                tracing::info!("Account is registered.");
            }

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
