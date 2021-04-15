use serde::{Deserialize, Serialize};

use crate::browser::*;
use crate::sway::types::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "application")]
pub enum ConnectionType {
    #[serde(rename = "browser")]
    Browser { id: String },
    #[serde(rename = "cli")]
    Cli,
}

impl PartialEq for ConnectionType {
    fn eq(&self, other: &Self) -> bool {
        use ConnectionType::*;
        match self {
            Browser { id: me } => match other {
                Browser { id: you } => me == you,
                _ => false,
            },
            _ => false,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "browser_request")]
pub enum BrowserRequest {
    #[serde(rename = "focus_tab")]
    FocusTab(BrowserTabRef),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "cli_request")]
pub enum CliRequest {
    #[serde(rename = "focus_tab")]
    FocusTab(BrowserTabRef),
    #[serde(rename = "focus_window")]
    FocusWindow { id: usize },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "client_type")]
pub enum DesktopdClient {
    #[serde(rename = "win")]
    Window { data: SwayWindow },
    #[serde(rename = "tab")]
    Tab { data: BrowserTab },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "msg_type")]
pub enum DesktopdMessage {
    #[serde(rename = "connect")]
    Connect(ConnectionType),

    #[serde(rename = "disconnect")]
    Disconnect(ConnectionType),

    #[serde(rename = "browser_message")]
    BrowserMessage { data: BrowserResponse },

    #[serde(rename = "browser_request")]
    BrowserRequest(BrowserRequest),

    #[serde(rename = "cli_request")]
    CliRequest(CliRequest),

    #[serde(rename = "client_list")]
    ClientList { data: Vec<DesktopdClient> },
}

impl ConnectionType {
    pub fn has_id(&self, other: &str) -> bool {
        match self {
            ConnectionType::Browser { ref id } => other == id,
            _ => false,
        }
    }

    pub fn is_browser(&self) -> bool {
        match self {
            ConnectionType::Browser { .. } => true,
            _ => false,
        }
    }
}
