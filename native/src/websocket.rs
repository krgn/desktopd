use crate::browser::*;
use crate::message::*;
use crate::state::{GlobalState, Tx};
use async_std::net::{SocketAddr, TcpListener, TcpStream};
use async_std::task;
use async_tungstenite::tungstenite::protocol::Message;
use futures::prelude::*;
use futures::{channel::mpsc::unbounded, future, pin_mut};
use log::{error, info};
use notify_rust::Notification;
use std::env;
use std::io;

// ░█▀█░█░█░█▀▄░█░░░▀█▀░█▀▀
// ░█▀▀░█░█░█▀▄░█░░░░█░░█░░
// ░▀░░░▀▀▀░▀▀░░▀▀▀░▀▀▀░▀▀▀

pub async fn run(state: GlobalState, sway_tx: Tx) -> Result<(), io::Error> {
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    info!("Listening on: {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        task::spawn(accept_connection(state.clone(), sway_tx.clone(), stream));
    }

    Ok(())
}

// ░█▀█░█▀▄░▀█▀░█░█░█▀█░▀█▀░█▀▀
// ░█▀▀░█▀▄░░█░░▀▄▀░█▀█░░█░░█▀▀
// ░▀░░░▀░▀░▀▀▀░░▀░░▀░▀░░▀░░▀▀▀

async fn accept_connection(state: GlobalState, sway_tx: Tx, stream: TcpStream) {
    let addr = stream
        .peer_addr()
        .expect("connected streams should have a peer address");

    info!("Peer address: {}", addr);

    let ws_stream = async_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    info!("New WebSocket connection: {}", addr);

    let (tx, rx) = unbounded();
    let (write, read) = ws_stream.split();

    {
        state.lock().unwrap().add_peer(addr, tx);
    }

    let answer_channel = rx
        .map(|msg| serde_json::to_string(&msg).unwrap())
        .map(|json| Ok(Message::Text(json)))
        .forward(write);

    let receive_handle = read
        .try_filter(|msg| future::ready(!msg.is_close()))
        .try_for_each(|msg| {
            let sway_handle = sway_tx.clone();
            let receive_state = state.clone();
            handle_message(receive_state, &addr, sway_handle, msg);
            future::ok(())
        });

    pin_mut!(receive_handle, answer_channel);
    future::select(answer_channel, receive_handle).await;

    info!("{} disconnected", &addr);
    if let Some((conn, _)) = state.lock().unwrap().remove_peer(&addr) {
        if conn.map(|t| t.is_browser()).unwrap_or(false) {
            show_notification("Browser Plugin disconnected")
        }
    }
}

fn show_notification(message: &str) {
    Notification::new()
        .summary("desktopd")
        .body(message)
        .show()
        .expect("Could not show notification");
}

fn handle_message(state: GlobalState, addr: &SocketAddr, sway_tx: Tx, msg: Message) {
    let txt = msg.to_text();

    if txt.is_err() {
        error!("Recieved non-string message from client");
        return;
    }

    let raw = txt.unwrap();
    let json = serde_json::from_str::<DesktopdMessage>(raw);

    if json.is_err() {
        error!("Received invalid json string: {:#?}", raw);
        error!("Parse error: {:#?}", json.err());
        return;
    }

    let msg: DesktopdMessage = json.unwrap();
    let inner_state = state.clone();
    let sway_handle = sway_tx.clone();
    handle_desktopd_message(inner_state, &addr, sway_handle, msg);
}

fn handle_desktopd_message(
    state: GlobalState,
    addr: &SocketAddr,
    sway_tx: Tx,
    msg: DesktopdMessage,
) {
    use DesktopdMessage::*;
    match msg {
        Connect(conn_type) => handle_connect(state, addr, conn_type),
        CliRequest(data) => handle_cli_request(state, sway_tx, data),
        BrowserMessage { data } => handle_browser_response(state, sway_tx, data),
        _ => {}
    }
}

fn handle_connect(state: GlobalState, addr: &SocketAddr, msg: ConnectionType) {
    let mut state = state.lock().unwrap();
    match msg {
        ConnectionType::Browser { id } => {
            show_notification("Browser plugin connected");
            state.mark_browser(id, &addr);
        }

        // a new cli has connected, send the current list of clients
        ConnectionType::Cli => {
            state.mark_cli(&addr);
            let clients = state.clients();
            let init = DesktopdMessage::ClientList { data: clients };
            let peer: Tx = state.find_peer(&addr).unwrap().clone();
            peer.unbounded_send(init)
                .expect("Could not respond with client list");
        }
    }
}

fn handle_browser_response(state: GlobalState, sway_tx: Tx, data: BrowserResponse) {
    let mut state = state.lock().unwrap();
    use BrowserResponse::*;
    match data {
        Init { data: tabs } => {
            info!("Received initial tab list from browser");
            for tab in tabs {
                state.add_tab(tab)
            }
        }

        Updated { data: tab } => {
            info!("Updated tab {}", tab.id);
            state.add_tab(tab)
        }

        Removed(tab) => {
            info!("Removed tab {}", tab.tab_id);
            state.remove_tab(tab)
        }

        Activated(_) => {
            sway_tx
                .unbounded_send(DesktopdMessage::BrowserMessage { data })
                .unwrap();
        }

        _ => (),
    }
}

fn handle_cli_request(state: GlobalState, sway_tx: Tx, data: CliRequest) {
    let mut state = state.lock().unwrap();

    use CliRequest::*;
    match &data {
        FocusWindow { .. } => {
            sway_tx
                .unbounded_send(DesktopdMessage::CliRequest(data))
                .unwrap();
        }

        FocusTab { .. } => {
            for (peer_addr, peer) in state.get_browser_connections() {
                match peer.unbounded_send(DesktopdMessage::CliRequest(data.clone())) {
                    Ok(_) => info!("Successfully sent focus-tab message to browsers"),
                    Err(e) => {
                        if let Some((conn, _)) = state.remove_peer(&peer_addr) {
                            if conn.map(|t| t.is_browser()).unwrap_or(false) {
                                show_notification("Browser Plugin disconnected")
                            }
                        }
                        error!("Could not send message to browser {}: {}", peer_addr, e)
                    }
                }
            }
        }
    }
}
