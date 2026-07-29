use async_trait::async_trait;
use storage::api::{
    encode,
    requests::{RemoveTableRequest, RemoveTableResponse},
};

use crate::client::{CliResult, PersistentClient};
use crate::commands::CommandHandler;
use crate::parser::parse_one_arg;

pub struct RemoveTableCommand;

#[async_trait]
impl CommandHandler for RemoveTableCommand {
    fn name(&self) -> &'static str {
        "remove_table"
    }

    fn usage(&self) -> &'static str {
        "remove_table <table>"
    }

    fn description(&self) -> &'static str {
        "Remove a table and all of its records"
    }

    async fn execute(&self, client: &mut PersistentClient, args: &[&str]) -> CliResult<()> {
        let table = parse_one_arg(args, self.usage())?;
        let req = RemoveTableRequest { table };
        let resp: RemoveTableResponse = client.send(self.name(), encode(&req)).await?;
        println!("ok: {}", resp.ok);
        Ok(())
    }
}
