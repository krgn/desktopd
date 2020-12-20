use async_std::net::SocketAddr;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::browser::*;
use crate::message::*;
use crate::sway::types::*;

pub type Tx = UnboundedSender<DesktopdMessage>;
pub type Rx = UnboundedReceiver<DesktopdMessage>;
pub type TabId = usize;
pub type WindowId = usize;

pub struct State {
    peers: HashMap<SocketAddr, (Option<ConnectionType>, Tx)>,
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

    pub fn add_peer(&mut self, addr: SocketAddr, tx: Tx) {
        self.peers.insert(addr, (None, tx));
    }

    pub fn mark_browser(&mut self, addr: &SocketAddr) {
        if let Some((_, tx)) = self.peers.remove(addr) {
            self.peers
                .insert(*addr, (Some(ConnectionType::Browser), tx));
        }
    }

    pub fn mark_cli(&mut self, addr: &SocketAddr) {
        if let Some((_, tx)) = self.peers.remove(addr) {
            self.peers.insert(*addr, (Some(ConnectionType::Cli), tx));
        }
    }

    pub fn remove_peer(&mut self, addr: &SocketAddr) {
        self.peers.remove(addr);
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
            if let Some(ConnectionType::Browser) = t {
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
        let tabs = self
            .tabs
            .iter()
            .flat_map(|(_, inner)| inner.iter().map(|(_, tabs)| tabs))
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
