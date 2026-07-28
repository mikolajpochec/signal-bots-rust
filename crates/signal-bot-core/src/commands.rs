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

    /// Try to route a message to a command.
    /// Returns `(Option<Result<(), BotError>>, Option<(MessageContext, Vec<String>)>)`.
    /// - If a command matched: `(Some(result), None)`
    /// - If no command matched: `(None, Some((ctx, args)))` — ctx is returned for plugin fallback
    /// - If the message doesn't start with the prefix: `(None, Some((ctx, vec![])))` — ctx is returned unmodified
    pub async fn route(&self, ctx: MessageContext) -> (Option<Result<(), BotError>>, Option<(MessageContext, Vec<String>)>) {
        if !ctx.text.starts_with(&self.prefix) {
            return (None, Some((ctx, vec![])));
        }

        let without_prefix = ctx.text[self.prefix.len()..].trim_start();
        if without_prefix.is_empty() {
            return (None, Some((ctx, vec![])));
        }

        let mut parts = without_prefix.split_whitespace();
        let trigger = match parts.next() {
            Some(t) => t.to_string(),
            None => return (None, Some((ctx, vec![]))),
        };
        let args: Vec<String> = parts.map(|s| s.to_string()).collect();

        if trigger == "help" {
            let help_text = self.help_text();
            let result = ctx.reply(&help_text).await;
            return (Some(result), None);
        }

        if let Some(command) = self.commands.get(&trigger) {
            let result = (command.handler)(ctx, args).await;
            (Some(result), None)
        } else {
            // No built-in command — return ctx + parsed trigger/args for plugin fallback
            (None, Some((ctx, {
                let mut v = vec![trigger];
                v.extend(args);
                v
            })))
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
