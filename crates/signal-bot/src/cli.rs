use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "signal-bot", about = "Signal Messenger bot framework")]
pub struct Cli {
    /// Increase log verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
    
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Start the bot using a configuration file
    Run {
        /// Path to bot.toml configuration file
        #[arg(short, long, default_value = "./bot.toml")]
        config: PathBuf,
    },
    
    /// Start the signal-cli daemon using settings from config
    Daemon {
        /// Path to bot.toml configuration file
        #[arg(short, long, default_value = "./bot.toml")]
        config: PathBuf,
    },
    
    /// Send a one-off message
    Send {
        /// signal-cli daemon socket path or TCP address
        #[arg(long, default_value = "/tmp/signal-cli.sock")]
        socket: String,
        
        /// Recipient phone number or UUID (for direct messages)
        #[arg(long, conflicts_with = "group")]
        recipient: Option<String>,
        
        /// Group ID (for group messages)
        #[arg(long, conflicts_with = "recipient")]
        group: Option<String>,
        
        /// Message text
        #[arg(short, long)]
        message: String,
    },
    
    /// Manage groups
    Groups {
        /// signal-cli daemon socket path or TCP address
        #[arg(long, default_value = "/tmp/signal-cli.sock")]
        socket: String,
        
        #[command(subcommand)]
        subcommand: GroupsCommand,
    },
    
    /// Show version information
    Version,
}

#[derive(Subcommand)]
pub enum GroupsCommand {
    /// List all groups
    List,
}
