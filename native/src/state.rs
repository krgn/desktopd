use crate::browser::*;
use crate::message::*;
use crate::sway::types::*;
use async_std::net::SocketAddr;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

pub type Tx = UnboundedSender<DesktopdMessage>;
pub type Rx = UnboundedReceiver<DesktopdMessage>;
pub type TabId = usize;
pub type WindowId = usize;

pub struct State {
    peers: HashMap<SocketAddr, (ConnectionType, Tx)>,
    tabs: HashMap<WindowId, HashMap<TabId, BrowserTab>>,
    windows: HashMap<WindowId, SwayWindow>,
}

impl State {
    pub fn new() -> State {
        State {
            peers: HashMap::new(),
            tabs: HashMap::new(),
            windows: HashMap::new(),
        }
    }

    pub fn add_peer(&mut self, tipe: ConnectionType, addr: SocketAddr, tx: Tx) {
        match tipe {
            ConnectionType::Browser { .. } => {
                self.peers.retain(|_, (inner_t, _)| &tipe != inner_t);
                self.peers.insert(addr, (tipe, tx));
            }
            ConnectionType::Cli => {
                self.peers.insert(addr, (tipe, tx));
            }
        }
    }

    pub fn remove_peer(&mut self, addr: &SocketAddr) -> Option<(ConnectionType, Tx)> {
        self.peers.remove(addr)
    }

    pub fn find_peer(&self, addr: &SocketAddr) -> Option<Tx> {
        if self.peers.contains_key(addr) {
            Some(self.peers[addr].1.clone())
        } else {
            None
        }
    }

    pub fn get_browser_windows(&self) -> Vec<&SwayWindow> {
        self.windows
            .iter()
            .filter(|(_, win)| win.is_browser())
            .map(|(_, win)| win)
            .collect::<Vec<&SwayWindow>>()
    }

    pub fn get_browser_connections(&self) -> Vec<(SocketAddr, Tx)> {
        self.peers.iter().fold(vec![], |mut out, (addr, (t, tx))| {
            if let ConnectionType::Browser { .. } = t {
                out.push((*addr, tx.clone()));
            }
            out
        })
    }

    pub fn get_focused(&self) -> Vec<&SwayWindow> {
        self.windows
            .iter()
            .filter(|(_, win)| win.focused)
            .map(|(_, win)| win)
            .collect::<Vec<&SwayWindow>>()
    }

    pub fn remove_focused(&mut self) -> Vec<SwayWindow> {
        let mut out = vec![];
        let ids = self
            .get_focused()
            .iter()
            .map(|win| win.id)
            .collect::<Vec<usize>>();

        for id in ids {
            if let Some(focused) = self.windows.remove(&id) {
                out.push(focused)
            }
        }
        out
    }

    pub fn add_window(&mut self, win: SwayWindow) {
        self.windows.insert(win.id, win);
    }

    pub fn remove_window(&mut self, id: &WindowId) {
        self.windows.remove(id);
    }

    pub fn clients(&self) -> Vec<DesktopdClient> {
        let window_titles = self
            .windows
            .iter()
            .map(|(_, win)| &win.name[..])
            .collect::<HashSet<&str>>();

        let tabs = self
            .tabs
            .iter()
            .flat_map(|(_, inner)| inner.iter().map(|(_, tabs)| tabs))
            .filter(|tab| {
                window_titles
                    .iter()
                    .fold(true, |result, name| result && !name.contains(&tab.title))
            })
            .map(|tab| DesktopdClient::Tab { data: tab.clone() })
            .collect::<Vec<DesktopdClient>>();

        let mut windows = self
            .windows
            .iter()
            .map(|(_, win)| DesktopdClient::Window { data: win.clone() })
            .collect::<Vec<DesktopdClient>>();

        windows.extend(tabs);
        windows
    }

    pub fn add_tab(&mut self, tab: BrowserTab) {
        if self.tabs.contains_key(&tab.window_id) {
            self.tabs.get_mut(&tab.window_id).map(|inner| {
                inner.insert(tab.id, tab);
            });
        } else {
            let mut map = HashMap::new();
            let window_id = tab.window_id;
            map.insert(tab.id, tab);
            self.tabs.insert(window_id, map);
        }
    }

    pub fn remove_tab(&mut self, tab: BrowserTabRef) {
        if let Some(mut tabs) = self.tabs.remove(&tab.window_id) {
            tabs.remove(&tab.tab_id);
            self.tabs.insert(tab.window_id, tabs);
        }
    }

    pub fn find_tab(&self, tab: &BrowserTabRef) -> Option<&BrowserTab> {
        self.tabs
            .get(&tab.window_id)
            .map(|tabs| tabs.get(&tab.tab_id))
            .flatten()
    }
}

pub type GlobalState = Arc<Mutex<State>>;
