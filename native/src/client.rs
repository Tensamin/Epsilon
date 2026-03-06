use crate::{CommunicationError, Receiver, Sender};
use rustls::{ClientConfig as RustlsClientConfig, RootCertStore};
use rustls_native_certs::load_native_certs;
use wtransport::{ClientConfig, Connection, Endpoint};

pub async fn connect(url: &str) -> Result<(Sender, Receiver), CommunicationError> {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    let client_config = configure_client()?;
    let endpoint = Endpoint::client(client_config)?;

    let connecting = endpoint.connect(url);

    let connection: Connection = connecting
        .await
        .map_err(|e| CommunicationError::ConnectingError(e))?;

    let sender = Sender::new(connection.clone());
    let receiver = Receiver::new(connection);

    Ok((sender, receiver))
}

fn configure_client() -> Result<ClientConfig, CommunicationError> {
    let mut root_store = RootCertStore::empty();

    let certs = load_native_certs().certs;

    for cert in certs {
        root_store.add(cert).ok();
    }

    let mut tls_config = RustlsClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    tls_config.alpn_protocols = vec![
        b"h3".to_vec(),
        b"h3-29".to_vec(),
        b"h3-28".to_vec(),
        b"h3-27".to_vec(),
    ];

    let client_config = ClientConfig::builder()
        .with_bind_default()
        .with_custom_tls(tls_config)
        .build();

    Ok(client_config)
}
