use std::sync::Arc;
use std::{env, println};

use engine::MemoryStore;
use trench::api::{run_server, SharedStore};

#[derive(Debug, PartialEq, Eq)]
pub struct CliOptions {
    pub bootstraped: bool,
    pub address: String,
    pub config_path: String,
    pub show_help: bool,
}

pub fn parse_args(args: &[String]) -> CliOptions {
    let show_help = args.iter().any(|arg| arg == "--help" || arg == "-h");
    let bootstraped = args.iter().any(|arg| arg == "--bootstrap");

    let address = args
        .iter()
        .position(|arg| arg == "--address")
        .and_then(|index| args.get(index + 1))
        .cloned()
        .or_else(|| {
            args.iter()
                .find(|arg| arg.starts_with("--address="))
                .and_then(|arg| arg.split_once('=').map(|(_, val)| val.to_string()))
        })
        .unwrap_or_else(|| "127.0.0.1:7878".to_string());

    let config_path = args
        .iter()
        .position(|arg| arg == "--config" || arg == "-c")
        .and_then(|index| args.get(index + 1))
        .cloned()
        .or_else(|| {
            args.iter()
                .find(|arg| arg.starts_with("--config=") || arg.starts_with("-c="))
                .and_then(|arg| arg.split_once('=').map(|(_, val)| val.to_string()))
        })
        .unwrap_or_else(|| "config.trench".to_string());

    CliOptions {
        bootstraped,
        address,
        config_path,
        show_help,
    }
}

fn print_help() {
    println!("Trench DB Storage Server\n");
    println!("Usage:");
    println!("  trench [OPTIONS]\n");
    println!("Options:");
    println!("  -c, --config <PATH>    Path to the node configuration file [default: config.trench]");
    println!("      --address <ADDR>   Address and port to bind [default: 127.0.0.1:7878]");
    println!("      --bootstrap        Run node in bootstrap mode");
    println!("  -h, --help             Print help information");
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let options = parse_args(&args);

    if options.show_help {
        print_help();
        return;
    }

    let addr: std::net::SocketAddr = options.address.parse().expect("invalid address");

    let store: SharedStore = Arc::new(MemoryStore::new());
    println!("[trench] starting storage server on {addr}");
    println!("[trench] bootstrap mode: {}", options.bootstraped);
    println!("[trench] config file: {}", options.config_path);

    if let Err(err) = run_server(addr, store, options.bootstraped, &options.config_path).await {
        eprintln!("[trench] storage server failed: {err}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_args() {
        let args = vec!["trench".to_string()];
        let opts = parse_args(&args);
        assert_eq!(opts.config_path, "config.trench");
        assert_eq!(opts.address, "127.0.0.1:7878");
        assert!(!opts.bootstraped);
        assert!(!opts.show_help);
    }

    #[test]
    fn test_custom_config_flag_space() {
        let args = vec![
            "trench".to_string(),
            "--config".to_string(),
            "nodes/node2.trench".to_string(),
        ];
        let opts = parse_args(&args);
        assert_eq!(opts.config_path, "nodes/node2.trench");
    }

    #[test]
    fn test_custom_config_short_flag_space() {
        let args = vec![
            "trench".to_string(),
            "-c".to_string(),
            "nodes/node2.trench".to_string(),
        ];
        let opts = parse_args(&args);
        assert_eq!(opts.config_path, "nodes/node2.trench");
    }

    #[test]
    fn test_custom_config_flag_equals() {
        let args = vec![
            "trench".to_string(),
            "--config=nodes/node2.trench".to_string(),
        ];
        let opts = parse_args(&args);
        assert_eq!(opts.config_path, "nodes/node2.trench");
    }

    #[test]
    fn test_custom_config_short_flag_equals() {
        let args = vec![
            "trench".to_string(),
            "-c=nodes/node2.trench".to_string(),
        ];
        let opts = parse_args(&args);
        assert_eq!(opts.config_path, "nodes/node2.trench");
    }

    #[test]
    fn test_combined_flags() {
        let args = vec![
            "trench".to_string(),
            "--bootstrap".to_string(),
            "--config".to_string(),
            "custom.trench".to_string(),
            "--address".to_string(),
            "127.0.0.1:9000".to_string(),
        ];
        let opts = parse_args(&args);
        assert!(opts.bootstraped);
        assert_eq!(opts.config_path, "custom.trench");
        assert_eq!(opts.address, "127.0.0.1:9000");
    }

    #[test]
    fn test_help_flag() {
        let args = vec!["trench".to_string(), "--help".to_string()];
        let opts = parse_args(&args);
        assert!(opts.show_help);

        let args_short = vec!["trench".to_string(), "-h".to_string()];
        let opts_short = parse_args(&args_short);
        assert!(opts_short.show_help);
    }
}

