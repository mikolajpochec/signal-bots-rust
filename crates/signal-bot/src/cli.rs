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
    

    /// Send a one-off message
    Send {
        /// Path to bot.toml configuration file
        #[arg(short, long, default_value = "./bot.toml")]
        config: PathBuf,
        /// Recipient phone number
        #[arg(short, long)]
        recipient: Option<String>,
        /// Recipient group ID
        #[arg(short, long)]
        group: Option<String>,
        /// Message text
        #[arg(short, long)]
        message: String,
    },
    
    /// Group management commands
    Groups {
        /// Path to bot.toml configuration file
        #[arg(short, long, default_value = "./bot.toml")]
        config: PathBuf,
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
