use serde::{Deserialize, Serialize};

use crate::browser::*;
use crate::sway::types::*;
use skim::prelude::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "application")]
pub enum ConnectionType {
    #[serde(rename = "browser")]
    Browser,
    #[serde(rename = "cli")]
    Cli,
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

impl SkimItem for DesktopdClient {
    fn text(&self) -> Cow<str> {
        use DesktopdClient::*;
        match self {
            Window { data } => Cow::Owned(format!(
                "{} {} {} {}",
                data.id, data.app_id, data.name, data.class,
            )),
            Tab { data } => Cow::Owned(format!(
                "{}.{} {} {}",
                data.id, data.window_id, data.title, data.url
            )),
        }
    }
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
