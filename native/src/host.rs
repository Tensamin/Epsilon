use crate::{CommunicationError, Receiver, Sender};
use wtransport::{
    ClientConfig, Connection, Endpoint, Identity, RecvStream, SendStream, ServerConfig,
};

use std::net::SocketAddr;
use std::sync::Arc;

// Server hosting using WebTransport
pub struct Host {
    incoming: tokio::sync::mpsc::Receiver<(Sender, Receiver)>,
    _task: tokio::task::JoinHandle<()>,
}

impl Host {
    pub async fn next(&mut self) -> Option<(Sender, Receiver)> {
        self.incoming.recv().await
    }
}

pub async fn host(port: u16) -> Result<Host, CommunicationError> {
    let server_config = configure_server(port).await?;
    let endpoint = Endpoint::server(server_config)?;

    let (incoming_tx, incoming_rx) = tokio::sync::mpsc::channel(16);

    let task = tokio::spawn(async move {
        // WebTransport server loop: accept incoming sessions
        loop {
            // Wait for incoming session
            let incoming_session = endpoint.accept().await;

            // Wait for session request (HTTP/3 upgrade negotiation)
            let incoming_request = match incoming_session.await {
                Ok(req) => req,
                Err(_) => continue,
            };

            // Accept the WebTransport session
            let connection = match incoming_request.accept().await {
                Ok(conn) => conn,
                Err(_) => continue,
            };

            let incoming_tx = incoming_tx.clone();
            tokio::spawn(handle_connection(connection, incoming_tx));
        }
    });

    Ok(Host {
        incoming: incoming_rx,
        _task: task,
    })
}

async fn handle_connection(
    connection: Connection,
    tx: tokio::sync::mpsc::Sender<(Sender, Receiver)>,
) {
    let sender = Sender::new(connection.clone());
    let receiver = Receiver::new(connection);

    // Non-blocking send - if channel full, connection is dropped
    let _ = tx.try_send((sender, receiver));
}

async fn configure_server(port: u16) -> Result<ServerConfig, CommunicationError> {
    // Generate self-signed identity for WebTransport
    // In production, use Identity::load_pemfiles("cert.pem", "key.pem").await?
    let identity = Identity::self_signed(["localhost", "127.0.0.1", "::1"])
        .map_err(|_| CommunicationError::CertificateGenerationFailed)?;

    let server_config = ServerConfig::builder()
        .with_bind_default(port)
        .with_identity(identity)
        .build();

    Ok(server_config)
}
