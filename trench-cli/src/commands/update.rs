use async_trait::async_trait;
use trench::api::{
    encode,
    requests::{UpdateRequest, UpdateResponse},
};

use crate::client::{CliResult, PersistentClient};
use crate::commands::CommandHandler;
use crate::parser::parse_three_or_more_args;

pub struct UpdateCommand;

#[async_trait]
impl CommandHandler for UpdateCommand {
    fn name(&self) -> &'static str {
        "update"
    }

    fn usage(&self) -> &'static str {
        "update <table> <key> <value>"
    }

    fn description(&self) -> &'static str {
        "Replace the value of an existing key in a table"
    }

    async fn execute(&self, client: &mut PersistentClient, args: &[&str]) -> CliResult<()> {
        let (table, key, value) = parse_three_or_more_args(args, self.usage())?;
        let req = UpdateRequest { table, key, value };
        let resp: UpdateResponse = client.send(self.name(), encode(&req)).await?;
        println!("ok: {}", resp.ok);
        Ok(())
    }
}
