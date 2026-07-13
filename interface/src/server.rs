use std::{error::Error, net::SocketAddr};

use byteser::ByteSerializable;
use byteser_derive::ByteSerializable;
use tokio::net::{TcpListener, TcpStream};
use transport::{
    errors::TransportError,
    frame::frame::Frametype,
    tcp::{connection::Connection, manager::{Role, StreamManager}},
};

#[derive(Debug, ByteSerializable)]
struct UserMessage {
    message: String,
}

#[derive(Debug, ByteSerializable)]
struct ServerResponse {
    response: String,
}

pub async fn run_server(addr: SocketAddr) -> Result<(), Box<dyn Error>> {
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

                let mut slice: &[u8] = &payload;
                let request: UserMessage = UserMessage::byte_deserialize(&mut slice)
                    .map_err(|msg| TransportError::InvalidFrame(format!("deserialization failed: {}", msg)))?;

                let response = ServerResponse {
                    response: format!("ECHO: {}", request.message),
                };

                let mut response_bytes = Vec::new();
                response.byte_serialize(&mut response_bytes);
                manager.send_data(stream_id, response_bytes).await?;
                manager.close_stream(stream_id).await?;
                manager.flush().await?;

                println!("[server {peer_addr}] responded on stream {stream_id}");
            }
            Frametype::Close => {
                println!("[server {peer_addr}] stream {stream_id} closed by client");
            }
            Frametype::Reset => {
                println!("[server {peer_addr}] stream {stream_id} reset by client");
            }
            _ => {}
        }
    }
}
