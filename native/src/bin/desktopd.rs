#![feature(async_closure)]

use async_i3ipc::{
    event::{Event, Subscribe},
    I3,
};
use std::env;
use std::io;

use async_std::net::{SocketAddr, TcpListener, TcpStream};
use async_std::task;
use async_tungstenite::tungstenite::protocol::Message;
use futures::prelude::*;
use log::info;

use futures::{
    channel::mpsc::{unbounded, UnboundedSender},
    future, pin_mut,
};
use notify_rust::Notification;

use desktopd::browser::*;
use desktopd::message::*;
use desktopd::sway::*;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

type Tx = UnboundedSender<DesktopdMessage>;
type TabId = usize;
type WindowId = usize;

struct State {
    peers: HashMap<SocketAddr, Tx>,
    tabs: HashMap<WindowId, HashMap<TabId, BrowserTab>>,
    windows: HashMap<WindowId, SwayWindow>,
}

impl State {
    fn new() -> State {
        State {
            peers: HashMap::new(),
            tabs: HashMap::new(),
            windows: HashMap::new(),
        }
    }

    fn add_peer(&mut self, addr: SocketAddr, tx: Tx) {
        self.peers.insert(addr, tx);
    }

    fn remove_peer(&mut self, addr: &SocketAddr) {
        self.peers.remove(addr);
    }

    fn find_peer(&self, addr: &SocketAddr) -> Option<Tx> {
        if self.peers.contains_key(addr) {
            Some(self.peers[addr].clone())
        } else {
            None
        }
    }

    fn add_window(&mut self, win: SwayWindow) {
        self.windows.insert(win.id, win);
    }

    fn remove_window(&mut self, id: &WindowId) {
        self.windows.remove(id);
    }

    fn clients(&self) -> Vec<DesktopdClient> {
        self.windows
            .iter()
            .map(|(_, win)| DesktopdClient::Window { data: win.clone() })
            .collect::<Vec<DesktopdClient>>()
    }

    fn add_tab(&mut self, tab: BrowserTab) {
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
}

type PeerMap = Arc<Mutex<State>>;

async fn run() -> Result<(), io::Error> {
    let _ = env_logger::try_init();
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());

    let state = PeerMap::new(Mutex::new(State::new()));

    let windows = SwayWindow::fetch().await;
    for win in windows {
        state.lock().unwrap().add_window(win);
    }

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    info!("Listening on: {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        task::spawn(accept_connection(state.clone(), stream));
    }

    Ok(())
}

async fn accept_connection(peer_map: PeerMap, stream: TcpStream) {
    let addr = stream
        .peer_addr()
        .expect("connected streams should have a peer address");

    info!("Peer address: {}", addr);

    let ws_stream = async_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    info!("New WebSocket connection: {}", addr);

    Notification::new()
        .summary("desktopd")
        .body("a new connection was made")
        .show()
        .expect("Could not show notification");

    let (tx, rx) = unbounded();
    let (write, read) = ws_stream.split();

    peer_map.lock().unwrap().add_peer(addr, tx);

    let answer_channel = rx
        .map(|msg| serde_json::to_string(&msg).unwrap())
        .map(|json| Ok(Message::Text(json)))
        .forward(write);

    let receive_handle = read
        .try_filter(|msg| future::ready(!msg.is_close()))
        .try_for_each(|msg| {
            let resp: DesktopdMessage = msg
                .to_text()
                .map(|txt: &str| {
                    info!("parsing desktopd msg: {}", txt);
                    serde_json::from_str(txt)
                })
                .expect("Could not parse message")
                .expect("Could not parse message");

            info!("received {:#?}", resp);

            let inner_peer_map = peer_map.clone();
            let peers = inner_peer_map.lock().unwrap();

            match resp {
                DesktopdMessage::Connect(ConnectionType::Cli) => {
                    let clients = peers.clients();
                    let init = DesktopdMessage::ClientList { data: clients };
                    let peer: Tx = peers.find_peer(&addr).unwrap().clone();
                    peer.unbounded_send(init).unwrap();
                }
                _ => {}
            }

            future::ok(())
        });

    pin_mut!(receive_handle, answer_channel);
    future::select(answer_channel, receive_handle).await;

    println!("{} disconnected", &addr);
    peer_map.lock().unwrap().remove_peer(&addr);
}

#[async_std::main]
async fn main() -> io::Result<()> {
    task::spawn(async {
        info!("ws server starting");
        run().await.expect("Yes.");
        info!("but what now?");
    });

    let mut i3 = I3::connect().await?;
    let _resp = i3.subscribe([Subscribe::Window]).await?;

    let _tree = i3.get_tree().await?;
    let mut listener = i3.listen();

    use Event::*;
    while let Ok(event) = listener.next().await {
        // match event {
        //     Workspace(ev) => info!("workspace change event {:?}", ev),
        //     Window(ev) => info!("window event {:?}", ev),
        //     Output(ev) => info!("output event {:?}", ev),
        //     Mode(ev) => info!("mode event {:?}", ev),
        //     BarConfig(ev) => info!("bar config update {:?}", ev),
        //     Binding(ev) => info!("binding event {:?}", ev),
        //     Shutdown(ev) => info!("shutdown event {:?}", ev),
        //     Tick(ev) => info!("tick event {:?}", ev),
        // }
    }

    Ok(())
}
