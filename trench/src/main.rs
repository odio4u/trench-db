mod config;
mod auth;


// use crate::config::loader::Node;
use std::env;


fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <config_file_path>", args[0]);
        std::process::exit(1);
    }

    let config_file_path = &args[1];  
    println!("Loading node configuration from: {}", config_file_path); 
    
    match config::loader::Node::load_node_from_file(config_file_path) {
        Ok(node) => {
            println!("{}", node);
        }
        Err(e) => eprintln!("Error loading nodes: {}", e),
    }
}
