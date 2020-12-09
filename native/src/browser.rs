use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct BrowserTab {
    pub id: usize,
    #[serde(rename = "windowId")]
    pub window_id: usize,
    pub title: String,
    pub url: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BrowserTabRef {
    #[serde(rename = "tabId")]
    tab_id: usize,
    #[serde(rename = "windowId")]
    window_id: usize,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum BrowserResponse {
    #[serde(rename = "init")]
    Init { data: Vec<BrowserTab> },

    #[serde(rename = "created")]
    Created { data: BrowserTab },

    #[serde(rename = "activated")]
    Activated(BrowserTabRef),

    #[serde(rename = "attached")]
    Attached {
        #[serde(rename = "tabId")]
        tab_id: usize,
        #[serde(rename = "newWindowId")]
        new_window_id: usize,
        #[serde(rename = "newPosition")]
        new_position: usize,
    },

    #[serde(rename = "detached")]
    Detached {
        #[serde(rename = "tabId")]
        tab_id: usize,
        #[serde(rename = "oldWindowId")]
        old_window_id: usize,
        #[serde(rename = "oldPosition")]
        old_position: usize,
    },

    #[serde(rename = "highlighted")]
    Highlighted {
        #[serde(rename = "tabIds")]
        tab_ids: Vec<usize>,
        #[serde(rename = "windowId")]
        window_id: usize,
    },

    #[serde(rename = "moved")]
    Moved {
        #[serde(rename = "tabId")]
        tab_id: usize,
        #[serde(rename = "windowId")]
        window_id: usize,
        #[serde(rename = "fromIndex")]
        from_index: usize,
        #[serde(rename = "toIndex")]
        to_index: usize,
    },

    #[serde(rename = "replaced")]
    Replaced {
        #[serde(rename = "addedTabId")]
        added_tab_id: usize,
        #[serde(rename = "removedTabId")]
        removed_tab_id: usize,
    },

    #[serde(rename = "updated")]
    Updated { data: BrowserTab },

    #[serde(rename = "removed")]
    Removed(BrowserTabRef),
}
