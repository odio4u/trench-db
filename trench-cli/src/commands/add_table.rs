use async_trait::async_trait;
use storage::api::{
    encode,
    requests::{AddTableRequest, AddTableResponse},
};

use crate::client::{CliResult, PersistentClient};
use crate::commands::CommandHandler;
use crate::parser::parse_one_arg;

pub struct AddTableCommand;

#[async_trait]
impl CommandHandler for AddTableCommand {
    fn name(&self) -> &'static str {
        "add_table"
    }

    fn usage(&self) -> &'static str {
        "add_table <table>"
    }

    fn description(&self) -> &'static str {
        "Create a new table"
    }

    async fn execute(&self, client: &mut PersistentClient, args: &[&str]) -> CliResult<()> {
        let table = parse_one_arg(args, self.usage())?;
        let req = AddTableRequest { table };
        let resp: AddTableResponse = client.send(self.name(), encode(&req)).await?;
        println!("ok: {}", resp.ok);
        Ok(())
    }
}
