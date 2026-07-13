use std::{error::Error, net::SocketAddr};

use transport::client::resilient_client::ResilientClient;
use byteser_derive::ByteSerializable;

#[derive(Debug, ByteSerializable)]
struct UserMessage {
    message: String,
}

#[derive(Debug, ByteSerializable)]
struct ServerResponse {
    response: String,
}

pub async fn resilient_client_run(addr: SocketAddr, message: String) -> Result<(), Box<dyn Error>> {
    let host = addr.ip().to_string();
    let port = addr.port();

    let mut client = ResilientClient::new(host, port);
    client.build_stream().await?;
    println!("[client] connected to {addr}");

    let request = UserMessage { message };
    let response: ServerResponse = client.send_message(&request).await?;

    println!("[client] response: {}", response.response);
    client.close().await?;
    Ok(())
}
