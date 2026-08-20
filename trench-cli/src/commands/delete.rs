use async_trait::async_trait;
use trench::api::{
    encode,
    requests::{DeleteRequest, DeleteResponse},
};

use crate::client::{CliResult, PersistentClient};
use crate::commands::CommandHandler;
use crate::parser::parse_two_args;

pub struct DeleteCommand;

#[async_trait]
impl CommandHandler for DeleteCommand {
    fn name(&self) -> &'static str {
        "delete"
    }

    fn usage(&self) -> &'static str {
        "delete <table> <key>"
    }

    fn description(&self) -> &'static str {
        "Remove a key from a table"
    }

    async fn execute(&self, client: &mut PersistentClient, args: &[&str]) -> CliResult<()> {
        let (table, key) = parse_two_args(args, self.usage())?;
        let req = DeleteRequest { table, key };
        let resp: DeleteResponse = client.send(self.name(), encode(&req)).await?;
        println!("ok: {}", resp.ok);
        Ok(())
    }
}
