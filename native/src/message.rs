use serde::{Serialize, Deserialize};

use crate::browser::*;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "application")]
pub enum ConnectionType {
    #[serde(rename = "browser")]
    Browser,
    #[serde(rename = "cli")]
    Cli
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum DesktopdResponse {
    #[serde(rename = "connection")]
    Connection(ConnectionType),

    #[serde(rename = "message")]
    BrowserMessage(BrowserResponse),
}
