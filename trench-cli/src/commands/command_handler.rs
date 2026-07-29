use async_trait::async_trait;

use crate::client::{CliResult, PersistentClient};

/// Trait implemented by every trench-cli command.
#[async_trait]
pub trait CommandHandler: Send + Sync {
    fn name(&self) -> &'static str;
    fn usage(&self) -> &'static str;
    fn description(&self) -> &'static str;
    async fn execute(&self, client: &mut PersistentClient, args: &[&str]) -> CliResult<()>;
}
