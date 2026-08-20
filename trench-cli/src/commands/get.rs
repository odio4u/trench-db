use async_trait::async_trait;
use trench::api::{
    encode,
    requests::{GetRequest, GetResponse},
};

use crate::client::{CliResult, PersistentClient};
use crate::commands::CommandHandler;
use crate::parser::parse_two_args;

pub struct GetCommand;

#[async_trait]
impl CommandHandler for GetCommand {
    fn name(&self) -> &'static str {
        "get"
    }

    fn usage(&self) -> &'static str {
        "get <table> <key>"
    }

    fn description(&self) -> &'static str {
        "Retrieve the value for a key in a table"
    }

    async fn execute(&self, client: &mut PersistentClient, args: &[&str]) -> CliResult<()> {
        let (table, key) = parse_two_args(args, self.usage())?;
        let req = GetRequest { table, key };
        let resp: GetResponse = client.send(self.name(), encode(&req)).await?;
        match resp.value {
            Some(value) => println!("{}", String::from_utf8_lossy(&value)),
            None => println!("(not found)"),
        }
        Ok(())
    }
}
