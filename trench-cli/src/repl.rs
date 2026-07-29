use std::io::{self, Write};

use tokio::io::{AsyncBufReadExt, BufReader};

use crate::client::{CliResult, PersistentClient};
use crate::registry::CommandRegistry;

pub async fn run_repl(client: &mut PersistentClient) -> CliResult<()> {
    let registry = CommandRegistry::new();
    let stdin = tokio::io::stdin();
    let mut stdout = io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    println!("trench-db CLI connected to {}:{}", client.host, client.port);
    println!("Type 'help' for available commands, 'quit' to exit.");

    loop {
        print!("trench> ");
        stdout.flush()?;
        line.clear();

        match reader.read_line(&mut line).await {
            Ok(0) => {
                println!();
                break;
            }
            Ok(_) => {}
            Err(err) => {
                eprintln!("[repl] failed to read input: {err}");
                continue;
            }
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        let name = parts[0].to_lowercase();

        match name.as_str() {
            "quit" | "exit" => break,
            "help" => {
                print_help(&registry);
                continue;
            }
            _ => {}
        }

        match registry.get(&name) {
            Some(handler) => {
                if let Err(err) = handler.execute(client, &parts[1..]).await {
                    eprintln!("error: {err}");
                }
            }
            None => {
                eprintln!("unknown command: {name}. Type 'help' for available commands.");
            }
        }
    }

    Ok(())
}

pub fn print_help(registry: &CommandRegistry) {
    println!("Available commands:");
    for handler in registry.list() {
        println!("  {:20} - {}", handler.usage(), handler.description());
    }
    println!("  {:20} - {}", "help", "Show this help message");
    println!("  {:20} - {}", "quit | exit", "Exit the CLI");
}
