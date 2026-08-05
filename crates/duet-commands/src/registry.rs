use crate::command::Command;
use crate::predicate::CommandContext;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("Command not found: {0}")]
    NotFound(String),
    #[error("Command already registered: {0}")]
    AlreadyExists(String),
    #[error("Precondition failed for command: {0}")]
    PreconditionFailed(String),
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
}

pub type CommandHandler =
    Arc<dyn Fn(&CommandContext, Value) -> Result<(), CommandError> + Send + Sync>;

/// Dynamic registry for command definitions and execution dispatch.
#[derive(Default)]
pub struct CommandRegistry {
    commands: HashMap<String, Command>,
    handlers: HashMap<String, CommandHandler>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new command along with its execution handler.
    pub fn register(
        &mut self,
        cmd: Command,
        handler: CommandHandler,
    ) -> Result<(), CommandError> {
        if self.commands.contains_key(&cmd.id) {
            return Err(CommandError::AlreadyExists(cmd.id));
        }
        let id = cmd.id.clone();
        self.commands.insert(id.clone(), cmd);
        self.handlers.insert(id, handler);
        Ok(())
    }

    /// Look up a registered command specification by ID.
    pub fn get(&self, id: &str) -> Option<&Command> {
        self.commands.get(id)
    }

    /// List all registered commands.
    pub fn list(&self) -> Vec<&Command> {
        self.commands.values().collect()
    }

    /// Dispatch a command by ID, validating its precondition against the current context.
    pub fn dispatch(
        &self,
        id: &str,
        ctx: &CommandContext,
        args: Value,
    ) -> Result<(), CommandError> {
        let cmd = self
            .commands
            .get(id)
            .ok_or_else(|| CommandError::NotFound(id.to_string()))?;

        if let Some(ref pred) = cmd.precondition {
            if !ctx.eval(pred) {
                return Err(CommandError::PreconditionFailed(id.to_string()));
            }
        }

        let handler = self
            .handlers
            .get(id)
            .ok_or_else(|| CommandError::NotFound(id.to_string()))?;

        handler(ctx, args)
    }
}
