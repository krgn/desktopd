use crate::message::*;
use crate::state::{GlobalState, Tx};
use async_std::net::{TcpListener, TcpStream};
use async_std::task;
use async_tungstenite::tungstenite::protocol::Message;
use futures::prelude::*;
use futures::{channel::mpsc::unbounded, future, pin_mut};
use log::info;
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

async fn accept_connection(state: GlobalState, sway_tx: Tx, stream: TcpStream) {
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

    state.lock().unwrap().add_peer(addr, tx);

    let answer_channel = rx
        .map(|msg| serde_json::to_string(&msg).unwrap())
        .map(|json| Ok(Message::Text(json)))
        .forward(write);

    let receive_handle = read
        .try_filter(|msg| future::ready(!msg.is_close()))
        .try_for_each(|msg| {
            let resp: DesktopdMessage = msg
                .to_text()
                //.map(serde_json::from_str)
                .map(|txt| {
                    println!("json: {}", txt);
                    serde_json::from_str(txt)
                })
                .expect("Could not parse message")
                .expect("Could not parse message");

            info!("received {:#?}", resp);

            let inner_peer_map = state.clone();
            let peers = inner_peer_map.lock().unwrap();

            match resp {
                // a new cli has connected, send the current list of clients
                DesktopdMessage::Connect(ConnectionType::Cli) => {
                    let clients = peers.clients();
                    let init = DesktopdMessage::ClientList { data: clients };
                    let peer: Tx = peers.find_peer(&addr).unwrap().clone();
                    peer.unbounded_send(init).unwrap();
                }
                DesktopdMessage::CliRequest(CliRequest::FocusWindow { .. }) => {
                    sway_tx.unbounded_send(resp).unwrap();
                }
                _ => {}
            }

            future::ok(())
        });

    pin_mut!(receive_handle, answer_channel);
    future::select(answer_channel, receive_handle).await;

    println!("{} disconnected", &addr);
    state.lock().unwrap().remove_peer(&addr);
}
