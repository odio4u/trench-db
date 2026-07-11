use std::{error::Error, net::SocketAddr};

use clap::{Parser, Subcommand};
use tokio::net::{TcpListener, TcpStream};
use transport::{
    errors::TransportError,
    frame::frame::Frametype,
    tcp::{connection::Connection, manager::{Role, StreamManager}},
};

#[derive(Parser)]
#[command(author, version, about = "Simple transport protocol application example", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Server {
        #[arg(short, long, default_value = "127.0.0.1:5000")]
        addr: SocketAddr,
    },
    Client {
        #[arg(short, long, default_value = "127.0.0.1:5000")]
        addr: SocketAddr,
        #[arg(short, long, default_value = "hello from interface")]
        message: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    match cli.command {
        Command::Server { addr } => run_server(addr).await?,
        Command::Client { addr, message } => run_client(addr, message).await?,
    }

    Ok(())
}

async fn run_server(addr: SocketAddr) -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind(addr).await?;
    println!("[server] listening on {addr}");

    loop {
        let (socket, peer_addr) = listener.accept().await?;
        println!("[server] accepted connection from {peer_addr}");

        tokio::spawn(async move {
            if let Err(err) = handle_connection(socket, peer_addr).await {
                eprintln!("[server {peer_addr}] connection error: {err}");
            }
        });
    }
}

async fn handle_connection(stream: TcpStream, peer_addr: SocketAddr) -> Result<(), TransportError> {
    let mut manager = StreamManager::new(Connection::new(stream), Role::Acceptor);

    loop {
        let frame = match manager.recv_frame().await {
            Ok(frame) => frame,
            Err(TransportError::ConnectionClosed) => {
                println!("[server {peer_addr}] client disconnected");
                return Ok(());
            }
            Err(err) => return Err(err),
        };

        let stream_id = frame.header.stream_id;
        match frame.header.frame_type {
            Frametype::Open => {
                println!("[server {peer_addr}] stream {stream_id} opened");
            }
            Frametype::Data => {
                let payload = manager.recv_data(stream_id).await?.unwrap_or_default();
                if payload.is_empty() {
                    continue;
                }

                println!(
                    "[server {peer_addr}] received {} byte(s) on stream {stream_id}",
                    payload.len(),
                );

                let response = format!("ECHO: {}", String::from_utf8_lossy(&payload));
                manager.send_data(stream_id, response.into_bytes()).await?;
                manager.close_stream(stream_id).await?;
                manager.flush().await?;
                println!("[server {peer_addr}] responded on stream {stream_id}");
            }
            Frametype::Close => {
                println!("[server {peer_addr}] stream {stream_id} closed by client");
            }
            _ => {}
        }
    }
}

async fn run_client(addr: SocketAddr, message: String) -> Result<(), Box<dyn Error>> {
    let tcp = TcpStream::connect(addr).await?;
    println!("[client] connected to {addr}");

    let mut manager = StreamManager::new(Connection::new(tcp), Role::Initiator);
    let stream_id = manager.open_stream().await?;
    println!("[client] opened stream {stream_id}");

    manager.send_data(stream_id, message.as_bytes().to_vec()).await?;
    manager.close_stream(stream_id).await?;
    manager.flush().await?;
    println!("[client] message sent, waiting for response...");

    let response_payload = loop {
        let frame = manager.recv_frame().await?;
        if frame.header.stream_id == stream_id && frame.header.frame_type == Frametype::Data {
            break frame.payload;
        }
    };

    println!("[client] response: {}", String::from_utf8_lossy(&response_payload));
    Ok(())
}
