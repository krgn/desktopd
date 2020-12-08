use futures::{SinkExt, StreamExt};

use async_std::io;
use async_std::prelude::*;
use async_std::task;
use async_tungstenite::async_std::connect_async;
use async_tungstenite::tungstenite::protocol::Message;

use desktopd::message::*;
use log::info;

async fn run() {
    let (stdin_tx, stdin_rx) = futures::channel::mpsc::unbounded();
    task::spawn(read_stdin(stdin_tx));

    let (ws_stream, _) = connect_async("ws://127.0.0.1:8080")
        .await
        .expect("Failed to connect");

    println!("WebSocket handshake has been successfully completed");

    let init = DesktopdMessage::Connect(ConnectionType::Cli);
    let msg = Message::Text(serde_json::to_string(&init).unwrap());

    let (mut write, mut read) = ws_stream.split();

    write.send(msg).await.expect("Could not send init message");

    let response = read.next().await.expect("No response").expect("Error");

    let msg: DesktopdMessage = response
        .to_text()
        .map(|txt| serde_json::from_str(txt))
        .expect("Could not parse")
        .expect("Could not parse");

    println!("msg: {:#?}", msg);
    info!("received: {:#?}", msg)
}

// Our helper method which will read data from stdin and send it along the
// sender provided.
async fn read_stdin(tx: futures::channel::mpsc::UnboundedSender<Message>) {
    let mut stdin = io::stdin();
    loop {
        let mut buf = vec![0; 1024];
        let n = match stdin.read(&mut buf).await {
            Err(_) | Ok(0) => break,
            Ok(n) => n,
        };
        buf.truncate(n);
        tx.unbounded_send(Message::binary(buf)).unwrap();
    }
}

fn main() {
    task::block_on(run())
}
