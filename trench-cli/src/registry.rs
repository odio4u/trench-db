use std::collections::HashMap;

use crate::commands::{
    add_table::AddTableCommand, command_handler::CommandHandler, contains::ContainsCommand,
    delete::DeleteCommand, get::GetCommand, put::PutCommand, remove_table::RemoveTableCommand,
    update::UpdateCommand,
};

/// Registry mapping command names to their handler implementations.
pub struct CommandRegistry {
    commands: HashMap<String, Box<dyn CommandHandler>>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        let handlers: Vec<Box<dyn CommandHandler>> = vec![
            Box::new(GetCommand),
            Box::new(PutCommand),
            Box::new(UpdateCommand),
            Box::new(DeleteCommand),
            Box::new(ContainsCommand),
            Box::new(AddTableCommand),
            Box::new(RemoveTableCommand),
        ];
        let mut commands = HashMap::new();
        for handler in handlers {
            commands.insert(handler.name().to_string(), handler);
        }
        Self { commands }
    }

    pub fn get(&self, name: &str) -> Option<&Box<dyn CommandHandler>> {
        self.commands.get(name)
    }

    pub fn list(&self) -> Vec<&Box<dyn CommandHandler>> {
        let mut handlers: Vec<_> = self.commands.values().collect();
        handlers.sort_by_key(|h| h.name());
        handlers
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}
