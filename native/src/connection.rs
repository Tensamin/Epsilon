use crate::CommunicationError;
use epsilon_core::CommunicationValue;
use wtransport::Connection;


pub const MAX_MESSAGE_SIZE: u64 = 1_000_000_000;

pub struct Sender {
    connection: Connection,
    _phantom: std::marker::PhantomData<CommunicationValue>,
}

impl Sender {
    pub fn new(connection: Connection) -> Self {
        Self {
            connection,
            _phantom: std::marker::PhantomData,
        }
    }

    pub async fn send(&self, data: &CommunicationValue) -> Result<(), CommunicationError> {
        // Open unidirectional stream (WebTransport handles stream creation)
        let mut stream = self.connection.open_uni().await?;

        let bytes = data.to_bytes();
        let len = bytes.len() as u64;

        if len > MAX_MESSAGE_SIZE {
            return Err(CommunicationError::MessageTooLarge);
        }

        // Write length prefix then data
        // wtransport streams implement AsyncWriteExt
        use tokio::io::AsyncWriteExt;
        stream.write_u32(len as u32).await?;
        stream.write_all(&bytes).await?;

        // Gracefully shutdown the send stream
        stream.finish().await?;

        Ok(())
    }

    pub fn close(&self) {
        // Close connection with error code 0
        self.connection
            .close(wtransport::VarInt::from_u32(0), b"sender closed");
    }
}

// Receiver implementation using WebTransport streams
pub struct Receiver {
    connection: Connection,
    _phantom: std::marker::PhantomData<CommunicationValue>,
}

impl Receiver {
    pub fn new(connection: Connection) -> Self {
        Self {
            connection,
            _phantom: std::marker::PhantomData,
        }
    }

    pub async fn receive(&self) -> Result<CommunicationValue, CommunicationError> {
        // Accept incoming unidirectional stream
        let mut stream = self
            .connection
            .accept_uni()
            .await
            .map_err(|e| CommunicationError::WConnectError(e));

        // Read length (u32 = 4 bytes)
        use tokio::io::AsyncReadExt;
        let len = stream.read_u32().await? as u64;

        if len > MAX_MESSAGE_SIZE {
            return Err(CommunicationError::MessageTooLarge);
        }

        let mut buf = vec![0u8; len as usize];
        stream.read_exact(&mut buf).await;

        let value = CommunicationValue::from_bytes(&buf)
            .ok_or(CommunicationError::ParseCommunicationValue)?;

        Ok(value)
    }

    pub fn close(&self) {
        self.connection
            .close(wtransport::VarInt::from_u32(0), b"receiver closed");
    }
}
