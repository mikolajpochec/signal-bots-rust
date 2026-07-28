use crate::context::MessageContext;
use crate::error::BotError;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

pub type CommandHandler = Box<
    dyn Fn(MessageContext, Vec<String>) -> Pin<Box<dyn Future<Output = Result<(), BotError>> + Send>>
        + Send
        + Sync,
>;

pub struct Command {
    pub trigger: String,
    pub description: String,
    pub handler: CommandHandler,
}

pub struct CommandRouter {
    prefix: String,
    commands: HashMap<String, Command>,
}

impl CommandRouter {
    pub fn new(prefix: &str) -> Self {
        Self {
            prefix: prefix.to_string(),
            commands: HashMap::new(),
        }
    }

    /// Register a command with a static text response
    pub fn register_static(&mut self, trigger: &str, description: &str, response: &str) {
        let response = response.to_string();
        self.register(
            trigger,
            description,
            Box::new(move |ctx, _args| {
                let resp = response.clone();
                Box::pin(async move { ctx.reply(&resp).await })
            }),
        );
    }

    /// Register a command with a dynamic handler
    pub fn register(&mut self, trigger: &str, description: &str, handler: CommandHandler) {
        self.commands.insert(
            trigger.to_string(),
            Command {
                trigger: trigger.to_string(),
                description: description.to_string(),
                handler,
            },
        );
    }

    /// Try to route a message to a command. Returns None if the message doesn't match any command.
    pub async fn route(&self, ctx: MessageContext) -> Option<Result<(), BotError>> {
        if !ctx.text.starts_with(&self.prefix) {
            return None;
        }

        let without_prefix = ctx.text[self.prefix.len()..].trim_start();
        if without_prefix.is_empty() {
            return None;
        }

        let mut parts = without_prefix.split_whitespace();
        let trigger = parts.next()?;
        let args: Vec<String> = parts.map(|s| s.to_string()).collect();

        if trigger == "help" {
            let help_text = self.help_text();
            let fut = async move { ctx.reply(&help_text).await };
            return Some(fut.await);
        }

        if let Some(command) = self.commands.get(trigger) {
            let fut = (command.handler)(ctx, args);
            Some(fut.await)
        } else {
            Some(Err(BotError::CommandNotFound(trigger.to_string())))
        }
    }

    /// Generate a help text listing all commands
    pub fn help_text(&self) -> String {
        let mut help = String::from("Available commands:\n");
        let mut commands: Vec<_> = self.commands.values().collect();
        commands.sort_by_key(|c| &c.trigger);
        for cmd in commands {
            help.push_str(&format!("{}{}: {}\n", self.prefix, cmd.trigger, cmd.description));
        }
        help.trim().to_string()
    }
}
