# Signal Bot Guidelines

- When writing Lua plugins for the Signal bot, NEVER hardcode the command prefix (e.g., `/command` or `!command`) in the `description` string. ALWAYS use the dynamic `{::prefix}` placeholder instead (e.g., `Format: {::prefix}command`).
- Make commits as small and self-contained as possible
