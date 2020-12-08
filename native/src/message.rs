use serde::{Serialize, Deserialize};

use crate::sway::*;
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
pub enum BrowserRequest {
    #[serde(rename = "focus_tab")]
    FocusTab(BrowserTabRef)
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum DesktopdClient {
    #[serde(rename = "win")]
    Window { data: SwayWindow },
    #[serde(rename = "tab")]
    Tab { data: BrowserTab },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum DesktopdMessage {
    #[serde(rename = "connect")]
    Connect(ConnectionType),

    #[serde(rename = "disconnect")]
    Disconnect(ConnectionType),

    #[serde(rename = "browser_message")]
    BrowserMessage { data: BrowserResponse },

    #[serde(rename = "browser_request")]
    BrowserRequest(BrowserRequest),

    #[serde(rename = "client_list")]
    ClientList { data: Vec<DesktopdClient> }
}
