mod cli;
mod config;
mod logging;

use clap::Parser;

// We assume these types will be provided by the parallel crates
// signal_bot_rpc::client::SignalCliClient
// signal_bot_core::engine::Engine
// signal_bot_core::commands::{CommandRouter, Command}

async fn is_account_registered(phone: &str) -> bool {
    // Reading accounts.json directly avoids hanging when the daemon already holds the file lock.
    let base_dirs = directories::ProjectDirs::from("", "", "signal-cli");
    
    // signal-cli usually stores data in ~/.local/share/signal-cli/data/accounts.json
    let data_dir = match base_dirs {
        Some(b) => b.data_dir().to_path_buf(),
        None => std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".local/share/signal-cli/data")
    };
    
    // On Linux it's often directly in ~/.local/share/signal-cli/data
    // but directories crate puts it in ~/.local/share/signal-cli
    // Let's just check the most common path directly:
    let accounts_path = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join(".local/share/signal-cli/data/accounts.json");
        
    if let Ok(contents) = std::fs::read_to_string(&accounts_path) {
        contents.contains(phone)
    } else {
        // Fallback if not found, assume true to let daemon try (preventing hangs)
        true
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
            cmd.arg("-u").arg(&phone)
               .arg("daemon")
               .arg("--socket").arg(&effective_socket)
               .arg("--receive-mode").arg("on-start");
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

            // Profile state struct for saving
            #[derive(serde::Serialize, serde::Deserialize, Default, PartialEq)]
            struct ProfileState {
                name: String,
                profile_picture: Option<String>,
            }

            // Sync profile
            {
                let config_path_obj = std::path::Path::new(&config);
                let parent_dir = config_path_obj.parent().unwrap_or(std::path::Path::new(""));
                let file_stem = config_path_obj.file_stem().unwrap_or_default().to_string_lossy();
                let state_file = parent_dir.join(format!(".{}.state.json", file_stem));
                
                let current_state = ProfileState {
                    name: config_data.bot.name.clone(),
                    profile_picture: config_data.bot.profile_picture.clone(),
                };
                
                let needs_update = if state_file.exists() {
                    if let Ok(content) = std::fs::read_to_string(&state_file) {
                        if let Ok(saved_state) = serde_json::from_str::<ProfileState>(&content) {
                            saved_state != current_state
                        } else { true }
                    } else { true }
                } else { true };
                
                if needs_update {
                    tracing::info!("Updating bot profile...");
                    let mut params = serde_json::Map::new();
                    params.insert("givenName".to_string(), serde_json::json!(config_data.bot.name));
                    
                    if let Some(ref avatar) = config_data.bot.profile_picture {
                        let avatar_path = std::fs::canonicalize(avatar)
                            .unwrap_or_else(|_| std::path::PathBuf::from(avatar));
                        params.insert("avatar".to_string(), serde_json::json!(avatar_path.to_string_lossy().to_string()));
                    } else {
                        params.insert("removeAvatar".to_string(), serde_json::json!(true));
                    }
                    
                    if let Err(e) = client.call("updateProfile", serde_json::Value::Object(params)).await {
                        tracing::warn!("Failed to update profile: {}", e);
                    } else {
                        if let Ok(state_json) = serde_json::to_string_pretty(&current_state) {
                            let _ = std::fs::write(&state_file, state_json);
                        }
                        tracing::info!("Profile updated successfully.");
                    }
                }
            }

            let mut router = signal_bot_core::commands::CommandRouter::new(&config_data.bot.prefix);
            
            for cmd in config_data.commands {
                router.register_static(&cmd.trigger, &cmd.description, &cmd.response);
            }

            // Load plugins if configured
            let plugin_manager = if let Some(plugins_cfg) = &config_data.plugins {
                let mut pm = signal_bot_plugins::PluginManager::new(&plugins_cfg.directory);
                match pm.load_all() {
                    Ok(count) => {
                        tracing::info!("Loaded {} Lua plugin(s)", count);
                        for (trigger, desc) in pm.list() {
                            router.add_external_help(trigger, desc);
                        }
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

            let allowed_groups: Vec<String> = config_data.groups.into_iter().map(|g| g.group_id).collect();

            let engine = signal_bot_core::engine::Engine::new(
                client.clone(),
                router,
                None,
                allowed_groups,
                config_data.bot.admins.clone(),
                plugin_manager,
            );
            engine.run().await?;
        },
        cli::Command::Send { config, recipient, group, message } => {
            let config_data = config::BotConfig::load(&config)?;
            let socket = config_data.account.effective_socket();
            
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
        cli::Command::Groups { config, subcommand } => {
            let config_data = config::BotConfig::load(&config)?;
            let socket = config_data.account.effective_socket();
            
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
