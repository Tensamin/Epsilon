use wtransport::{ClientConfig, Connection, Endpoint};

use crate::{CommunicationError, Receiver, Sender};

use std::net::SocketAddr;
use std::sync::Arc;

pub async fn connect(url: &str) -> Result<(Sender, Receiver), CommunicationError> {
    let client_config = configure_client();
    let endpoint = Endpoint::client(client_config)?;

    let connecting = endpoint.connect(url);

    let connection: Connection = connecting
        .await
        .map_err(|e| CommunicationError::ConnectingError(e))?;

    let sender = Sender::new(connection.clone());
    let receiver = Receiver::new(connection);

    Ok((sender, receiver))
}

fn configure_client() -> ClientConfig {
    ClientConfig::builder().with_bind_default().build()
}
