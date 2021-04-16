use crate::browser::*;
use crate::error::*;
use crate::message::*;
use crate::state::{GlobalState, Tx};
use anyhow::Result;
use async_std::net::{SocketAddr, TcpListener, TcpStream};
use async_std::task;
use async_tungstenite::tungstenite::protocol::Message;
use futures::prelude::*;
use futures::{channel::mpsc::unbounded, channel::mpsc::UnboundedSender, future, pin_mut};
use log::{error, info};
use notify_rust::Notification;
use std::env;

// ░█▀█░█░█░█▀▄░█░░░▀█▀░█▀▀
// ░█▀▀░█░█░█▀▄░█░░░░█░░█░░
// ░▀░░░▀▀▀░▀▀░░▀▀▀░▀▀▀░▀▀▀

pub async fn run(state: GlobalState, sway_tx: Tx) -> Result<(), DesktopdError> {
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());

    // Create the event loop and TCP listener we'll accept connections on.
    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|err| DesktopdError::IoError(err))?;

    info!("Listening on: {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        task::spawn(accept_connection(state.clone(), sway_tx.clone(), stream));
    }

    Ok(())
}

// ░█▀█░█▀▄░▀█▀░█░█░█▀█░▀█▀░█▀▀
// ░█▀▀░█▀▄░░█░░▀▄▀░█▀█░░█░░█▀▀
// ░▀░░░▀░▀░▀▀▀░░▀░░▀░▀░░▀░░▀▀▀

async fn accept_connection(
    state: GlobalState,
    sway_tx: Tx,
    stream: TcpStream,
) -> Result<(), DesktopdError> {
    let addr = stream
        .peer_addr()
        .map_err(|err| DesktopdError::IoError(err))?;

    info!("Peer address: {}", addr);

    let ws_stream = async_tungstenite::accept_async(stream)
        .await
        .map_err(|err| DesktopdError::WebSocketError(err))?;

    info!("New WebSocket connection: {}", addr);

    let (tx, rx) = unbounded();
    let (write, mut read) = ws_stream.split();

    info!("Waiting for init message from {}", addr);

    let init = read
        .next()
        .await
        .map(|result| {
            result
                .map_err(|err| DesktopdError::WebSocketError(err))
                .and_then(|msg| {
                    msg.to_text()
                        .map(|txt| txt.to_owned())
                        .map_err(|err| DesktopdError::WebSocketError(err))
                })
                .and_then(|txt| {
                    serde_json::from_str::<DesktopdMessage>(&txt)
                        .map_err(|err| DesktopdError::SerializationError(err))
                })
        })
        .unwrap_or(Err(DesktopdError::ConnectInitError))?;

    info!("Init message received for {}: {:#?}", addr, &init);

    let init_state = state.clone();
    handle_init_message(init_state, &addr, tx, init)?;

    let answer_channel = rx
        .map(|msg| serde_json::to_string(&msg).unwrap())
        .map(|json| Ok(Message::Text(json)))
        .forward(write);

    let receive_handle = read
        .try_filter(|msg| future::ready(!msg.is_close()))
        .try_for_each(|msg| {
            let sway_handle = sway_tx.clone();
            let receive_state = state.clone();
            handle_message(receive_state, sway_handle, msg);
            future::ok(())
        });

    pin_mut!(receive_handle, answer_channel);
    future::select(answer_channel, receive_handle).await;

    info!("{} disconnected", &addr);
    if let Some((conn, _)) = state.lock().unwrap().remove_peer(&addr) {
        if conn.is_browser() {
            show_notification("Browser Plugin disconnected");
        }
    }

    return Ok(());
}

fn show_notification(message: &str) {
    Notification::new()
        .summary("desktopd")
        .body(message)
        .show()
        .expect("Could not show notification");
}

fn handle_message(state: GlobalState, sway_tx: Tx, msg: Message) {
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
    handle_desktopd_message(inner_state, sway_handle, msg);
}

fn handle_desktopd_message(state: GlobalState, sway_tx: Tx, msg: DesktopdMessage) {
    use DesktopdMessage::*;
    match msg {
        CliRequest(data) => handle_cli_request(state, sway_tx, data),
        BrowserMessage { data } => handle_browser_response(state, sway_tx, data),
        _ => {}
    }
}

fn handle_init_message(
    state: GlobalState,
    addr: &SocketAddr,
    tx: UnboundedSender<DesktopdMessage>,
    msg: DesktopdMessage,
) -> Result<(), DesktopdError> {
    use DesktopdMessage::*;
    match msg {
        Connect(conn_type) => handle_connect(state, addr, tx, conn_type),
        _ => Err(DesktopdError::ConnectInitError),
    }
}

fn handle_connect(
    state: GlobalState,
    addr: &SocketAddr,
    tx: UnboundedSender<DesktopdMessage>,
    tipe: ConnectionType,
) -> Result<(), DesktopdError> {
    use ConnectionType::*;
    let mut state = state.lock().unwrap();
    match tipe {
        Browser { .. } => {
            show_notification("Browser plugin connected");
            state.add_peer(tipe, *addr, tx);
            Ok(())
        }

        // a new cli has connected, send the current list of clients
        Cli => {
            state.add_peer(tipe, *addr, tx);
            let clients = state.clients();
            let init = DesktopdMessage::ClientList { data: clients };
            let peer: Tx = state
                .find_peer(&addr)
                .map(|handle| Ok(handle.clone()))
                .unwrap_or(Err(DesktopdError::ConnectInitError))?;
            peer.unbounded_send(init)
                .map_err(|err| DesktopdError::ChannelError(err))
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
                            if conn.is_browser() {
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
