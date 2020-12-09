use serde::{Deserialize, Serialize};

use crate::browser::*;
use crate::sway::*;
use skim::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "application")]
pub enum ConnectionType {
    #[serde(rename = "browser")]
    Browser,
    #[serde(rename = "cli")]
    Cli,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum BrowserRequest {
    #[serde(rename = "focus_tab")]
    FocusTab(BrowserTabRef),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
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
            Window { data } => Cow::Borrowed(&data.class),
            Tab { data } => Cow::Borrowed(&data.title),
        }
    }

    fn preview(&self, _context: PreviewContext) -> ItemPreview {
        use DesktopdClient::*;
        match self {
            Window { data } => ItemPreview::Text(format!("hello:\n{}", data.class)),
            Tab { data } => ItemPreview::Text(format!("hello:\n{}", data.title)),
        }
    }
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
    ClientList { data: Vec<DesktopdClient> },
}
