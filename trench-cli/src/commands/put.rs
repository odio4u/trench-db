use async_trait::async_trait;
use trench::api::{
    encode,
    requests::{PutRequest, PutResponse},
};

use crate::client::{CliResult, PersistentClient};
use crate::commands::CommandHandler;
use crate::parser::parse_three_or_more_args;

pub struct PutCommand;

#[async_trait]
impl CommandHandler for PutCommand {
    fn name(&self) -> &'static str {
        "put"
    }

    fn usage(&self) -> &'static str {
        "put <table> <key> <value>"
    }

    fn description(&self) -> &'static str {
        "Insert or overwrite a key-value pair in a table"
    }

    async fn execute(&self, client: &mut PersistentClient, args: &[&str]) -> CliResult<()> {
        let (table, key, value) = parse_three_or_more_args(args, self.usage())?;
        let req = PutRequest { table, key, value };
        let resp: PutResponse = client.send(self.name(), encode(&req)).await?;
        println!("ok: {}", resp.ok);
        Ok(())
    }
}
