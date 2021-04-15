use thiserror::Error;

#[derive(Error, Debug)]
pub enum DesktopdError {
    #[error(transparent)]
    WebSocketError(#[from] async_tungstenite::tungstenite::Error),

    #[error(transparent)]
    SerializationError(#[from] serde_json::Error),

    #[error(transparent)]
    IOError(#[from] std::io::Error),
}
