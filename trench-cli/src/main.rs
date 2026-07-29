use clap::{Parser, Subcommand};

use trench_cli::client::{CliResult, PersistentClient};
use trench_cli::registry::CommandRegistry;
use trench_cli::repl::run_repl;

#[derive(Parser)]
#[command(name = "trench-cli")]
#[command(about = "CLI client for the trench-db storage server")]
struct Cli {
    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    host: String,

    #[arg(short, long, default_value = "7878")]
    port: u16,

    #[command(subcommand)]
    command: Option<CliCommand>,
}

#[derive(Subcommand)]
enum CliCommand {
    #[command(name = "get")]
    Get { table: String, key: String },
    #[command(name = "put")]
    Put {
        table: String,
        key: String,
        value: String,
    },
    #[command(name = "update")]
    Update {
        table: String,
        key: String,
        value: String,
    },
    #[command(name = "delete")]
    Delete { table: String, key: String },
    #[command(name = "contains")]
    Contains { table: String, key: String },
    #[command(name = "add_table")]
    AddTable { table: String },
    #[command(name = "remove_table")]
    RemoveTable { table: String },
}

fn cli_command_to_args(cmd: &CliCommand) -> Vec<String> {
    match cmd {
        CliCommand::Get { table, key } => vec!["get".to_string(), table.clone(), key.clone()],
        CliCommand::Put { table, key, value } => vec!["put".to_string(), table.clone(), key.clone(), value.clone()],
        CliCommand::Update { table, key, value } => vec!["update".to_string(), table.clone(), key.clone(), value.clone()],
        CliCommand::Delete { table, key } => vec!["delete".to_string(), table.clone(), key.clone()],
        CliCommand::Contains { table, key } => vec!["contains".to_string(), table.clone(), key.clone()],
        CliCommand::AddTable { table } => vec!["add_table".to_string(), table.clone()],
        CliCommand::RemoveTable { table } => vec!["remove_table".to_string(), table.clone()],
    }
}

#[tokio::main]
async fn main() -> CliResult<()> {
    let cli = Cli::parse();
    let mut client = PersistentClient::new(cli.host.clone(), cli.port);

    if let Some(cmd) = cli.command {
        let registry = CommandRegistry::new();
        let args = cli_command_to_args(&cmd);
        let name = args[0].clone();
        let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        match registry.get(&name) {
            Some(handler) => handler.execute(&mut client, &arg_refs[1..]).await?,
            None => eprintln!("unknown command: {name}"),
        }
    } else {
        run_repl(&mut client).await?;
    }

    client.close().await?;
    Ok(())
}
