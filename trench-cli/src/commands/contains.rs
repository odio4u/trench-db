use async_trait::async_trait;
use storage::api::{
    encode,
    requests::{ContainsRequest, ContainsResponse},
};

use crate::client::{CliResult, PersistentClient};
use crate::commands::CommandHandler;
use crate::parser::parse_two_args;

pub struct ContainsCommand;

#[async_trait]
impl CommandHandler for ContainsCommand {
    fn name(&self) -> &'static str {
        "contains"
    }

    fn usage(&self) -> &'static str {
        "contains <table> <key>"
    }

    fn description(&self) -> &'static str {
        "Check whether a key exists in a table"
    }

    async fn execute(&self, client: &mut PersistentClient, args: &[&str]) -> CliResult<()> {
        let (table, key) = parse_two_args(args, self.usage())?;
        let req = ContainsRequest { table, key };
        let resp: ContainsResponse = client.send(self.name(), encode(&req)).await?;
        println!("{}", resp.exists);
        Ok(())
    }
}
