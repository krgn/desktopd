use crate::message::DesktopdMessage;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DesktopdError {
    #[error("Could not initialize connection.")]
    ConnectInitError,

    #[error(transparent)]
    WebSocketError(#[from] async_tungstenite::tungstenite::Error),

    #[error(transparent)]
    SerializationError(#[from] serde_json::Error),

    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error(transparent)]
    ChannelError(#[from] futures::channel::mpsc::TrySendError<DesktopdMessage>),
}
