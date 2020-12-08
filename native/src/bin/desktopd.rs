#![feature(async_closure)]

use async_i3ipc::{
    event::{Event, Subscribe},
    I3,
};
use std::io;
use std::env;

use async_std::net::{TcpListener, TcpStream};
use async_std::task;
use futures::prelude::*;
use log::info;

use notify_rust::Notification;
use futures::future;

use desktopd::message::*;

async fn run() -> Result<(), io::Error> {
    let _ = env_logger::try_init();
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    info!("Listening on: {}", addr);

    while let Ok((stream, _)) = listener.accept().await {
        task::spawn(accept_connection(stream));
    }

    Ok(())
}

async fn accept_connection(stream: TcpStream) {
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

    let (_write, read) = ws_stream.split();
    
    let handle = read
        .try_filter(|msg| {
            future::ready(!msg.is_close())
        })
        .try_for_each(|msg| {
            let resp: DesktopdResponse = msg
                .to_text()
                .map(|txt: &str| {
                    info!("parsing desktopd msg: {}", txt);
                    serde_json::from_str(txt)
                })
                .expect("Could not parse message")
                .expect("Could not parse message"); 

            info!("received {:#?}", resp);

            future::ok(())
        });
    
    handle.await.expect("Error during connection")
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
