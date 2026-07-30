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
    pub prefix: String,
    commands: HashMap<String, Command>,
    pub external_helps: Vec<(String, String, Vec<(String, String)>, String)>,
}

impl CommandRouter {
    pub fn new(prefix: &str) -> Self {
        Self {
            prefix: prefix.to_string(),
            commands: HashMap::new(),
            external_helps: Vec::new(),
        }
    }

    pub fn register_static(&mut self, trigger: &str, description: &str, response: &str) {
        let response = response.to_string();
        self.register(
            trigger,
            description,
            Box::new(move |ctx, args| {
                let mut resp = response.clone();
                
                // Replace {args} with all arguments joined together
                resp = resp.replace("{args}", &args.join(" "));
                
                // Replace {1}, {2}, etc. with specific arguments
                for (i, arg) in args.iter().enumerate() {
                    resp = resp.replace(&format!("{{{}}}", i + 1), arg);
                }
                
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
            (None, Some((ctx, {
                let mut v = vec![trigger];
                v.extend(args);
                v
            })))
        }
    }

    /// Add an external command (e.g. plugin) to the help text.
    pub fn add_external_help(&mut self, trigger: &str, description: &str, aliases: Vec<(String, String)>, group: &str) {
        self.external_helps.push((trigger.to_string(), description.to_string(), aliases, group.to_string()));
    }

    /// Generate a help text listing all commands
    pub fn help_text(&self) -> String {
        let mut help = String::from("Available commands:\n");
        
        let mut grouped: HashMap<String, Vec<(&String, &String, Vec<(String, String)>)>> = HashMap::new();
        
        // Built-in commands
        let mut core_cmds: Vec<_> = self.commands.values()
            .map(|c| (&c.trigger, &c.description, Vec::<(String, String)>::new()))
            .collect();
        core_cmds.sort_by(|a, b| a.0.cmp(b.0));
        grouped.insert("Core".to_string(), core_cmds);
        
        // External plugins
        for (trigger, desc, aliases, group) in &self.external_helps {
            grouped.entry(group.clone()).or_default().push((trigger, desc, aliases.clone()));
        }
        
        let mut group_names: Vec<_> = grouped.keys().collect();
        group_names.sort();
        
        for group in group_names {
            let cmds = grouped.get(group).unwrap();
            if cmds.is_empty() { continue; }
            
            help.push_str(&format!("\n### {}\n", group));
            let mut sorted_cmds = cmds.clone();
            sorted_cmds.sort_by(|a, b| a.0.cmp(b.0));
            
            for (trigger, desc, aliases) in sorted_cmds {
                let desc_formatted = desc.replace("{::prefix}", &self.prefix);
                help.push_str(&format!("{}{}: {}\n", self.prefix, trigger, desc_formatted));
                for (alias, alias_desc) in aliases {
                    let alias_desc_formatted = alias_desc.replace("{::prefix}", &self.prefix);
                    help.push_str(&format!("\t{}{}: {}\n", self.prefix, alias, alias_desc_formatted));
                }
            }
        }

        help.trim().to_string()
    }
}
