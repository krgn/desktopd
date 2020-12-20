use crate::browser::*;
use crate::message::*;
use crate::state::{GlobalState, Tx};
use async_std::net::{TcpListener, TcpStream};
use async_std::task;
use async_tungstenite::tungstenite::protocol::Message;
use futures::prelude::*;
use futures::{channel::mpsc::unbounded, future, pin_mut};
use log::{error, info};
use notify_rust::Notification;
use std::env;
use std::io;

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

fn show_notification(message: &str) {
    Notification::new()
        .summary("desktopd")
        .body(message)
        .show()
        .expect("Could not show notification");
}

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

    state.lock().unwrap().add_peer(addr, tx);

    let answer_channel = rx
        .map(|msg| serde_json::to_string(&msg).unwrap())
        .map(|json| Ok(Message::Text(json)))
        .forward(write);

    let receive_handle = read
        .try_filter(|msg| future::ready(!msg.is_close()))
        .try_for_each(|msg| {
            let txt = msg.to_text();

            if txt.is_err() {
                error!("Recieved non-string message from client");
                return future::ok(());
            }

            let raw = txt.unwrap();
            let json = serde_json::from_str::<DesktopdMessage>(raw);

            if json.is_err() {
                error!("Received invalid json string: {:#?}", raw);
                error!("Parse error: {:#?}", json.err());
                return future::ok(());
            }

            let resp: DesktopdMessage = json.unwrap();

            let inner_peer_map = state.clone();
            let mut peers = inner_peer_map.lock().unwrap();

            match resp {
                DesktopdMessage::Connect(ConnectionType::Browser) => {
                    show_notification("Browser plugin connected");
                    peers.mark_browser(&addr);
                }
                // a new cli has connected, send the current list of clients
                DesktopdMessage::Connect(ConnectionType::Cli) => {
                    peers.mark_cli(&addr);
                    let clients = peers.clients();
                    let init = DesktopdMessage::ClientList { data: clients };
                    let peer: Tx = peers.find_peer(&addr).unwrap().clone();
                    peer.unbounded_send(init).unwrap();
                }

                DesktopdMessage::CliRequest(CliRequest::FocusWindow { .. }) => {
                    sway_tx.unbounded_send(resp).unwrap();
                }

                DesktopdMessage::CliRequest(CliRequest::FocusTab { .. }) => {
                    for (peer_addr, peer) in peers.get_browser_connections() {
                        match peer.unbounded_send(resp.clone()) {
                            Ok(_) => info!("Successfully sent focus-tab message to browsers"),
                            Err(e) => {
                                peers.remove_peer(&peer_addr);
                                error!("Could not send message to browser {}: {}", peer_addr, e)
                            }
                        }
                    }
                }

                DesktopdMessage::BrowserMessage {
                    data: BrowserResponse::Init { data: tabs },
                } => {
                    for tab in tabs {
                        peers.add_tab(tab)
                    }
                }

                DesktopdMessage::BrowserMessage {
                    data: BrowserResponse::Updated { data: tab },
                } => peers.add_tab(tab),

                DesktopdMessage::BrowserMessage {
                    data: BrowserResponse::Removed(tab),
                } => peers.remove_tab(tab),

                DesktopdMessage::BrowserMessage {
                    data: BrowserResponse::Activated(_),
                } => {
                    sway_tx.unbounded_send(resp).unwrap();
                }

                _ => {}
            }

            future::ok(())
        });

    pin_mut!(receive_handle, answer_channel);
    future::select(answer_channel, receive_handle).await;

    info!("{} disconnected", &addr);
    state.lock().unwrap().remove_peer(&addr);
}
