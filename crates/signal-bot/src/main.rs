mod cli;
mod config;
mod logging;

use clap::Parser;

// We assume these types will be provided by the parallel crates
// signal_bot_rpc::client::SignalCliClient
// signal_bot_core::engine::Engine
// signal_bot_core::commands::{CommandRouter, Command}

async fn is_account_registered(phone: &str) -> bool {
    if let Ok(output) = tokio::process::Command::new("signal-cli").arg("listAccounts").output().await {
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout.contains(phone)
    } else {
        false
    }
}
async fn interactive_registration(phone: &str) -> anyhow::Result<()> {
    use std::io::Write;
    
    println!("⚠️  Bot is not registered with Signal yet.");
    println!("Attempting to register account {}...", phone);
    
    // First try without captcha
    let output = tokio::process::Command::new("signal-cli")
        .arg("-u").arg(phone).arg("register").output().await?;
        
    if output.status.success() {
        println!("✅ Registration request sent successfully.");
    } else {
        let err_str = String::from_utf8_lossy(&output.stderr).to_lowercase();
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
            let retry_output = tokio::process::Command::new("signal-cli")
                .arg("-u").arg(phone).arg("register").arg("--captcha").arg(captcha).output().await?;
                
            if !retry_output.status.success() {
                anyhow::bail!("Failed to register: {}", String::from_utf8_lossy(&retry_output.stderr));
            }
            println!("✅ Registration request sent successfully.");
        } else {
            anyhow::bail!("Failed to register: {}", err_str);
        }
    }

    print!("Enter the SMS verification code you received: ");
    std::io::stdout().flush()?;
    
    let mut code = String::new();
    std::io::stdin().read_line(&mut code)?;
    let code = code.trim().replace("-", ""); // Strip dashes if any

    println!("Verifying code...");
    let verify_output = tokio::process::Command::new("signal-cli")
        .arg("-u").arg(phone).arg("verify").arg(&code).output().await?;
        
    if !verify_output.status.success() {
        anyhow::bail!("Verification failed: {}", String::from_utf8_lossy(&verify_output.stderr));
    }
    
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
            
            // Clean up the socket if it was left behind by a previous crash
            if !effective_socket.starts_with("tcp://") {
                let _ = std::fs::remove_file(&effective_socket);
            }
            
            let mut cmd = tokio::process::Command::new("signal-cli");
            cmd.arg("-u").arg(&phone).arg("daemon").arg("--socket").arg(&effective_socket);
            cmd.kill_on_drop(true); // Automatically kill the daemon when the bot exits
            
            let mut daemon_child = cmd.spawn()?;
            
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
            if !is_account_registered(&phone).await {
                tracing::warn!("Account {} is not registered. Starting interactive registration...", phone);
                
                // Kill the background daemon so we can run CLI registration commands without DB locks
                daemon_child.kill().await.ok();
                
                interactive_registration(&config_data.account.phone).await?;
                
                println!("✅ Registration complete! Please restart the bot to connect.");
                std::process::exit(0);
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
