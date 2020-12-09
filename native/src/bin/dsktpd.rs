use futures::{SinkExt, StreamExt};

use async_std::io;
use async_std::prelude::*;
use async_std::task;
use async_tungstenite::async_std::connect_async;
use async_tungstenite::tungstenite::protocol::Message;
use skim::prelude::*;

use desktopd::message::*;
use desktopd::sway::*;

async fn run(tx_item: SkimItemSender) {
    let (ws_stream, _) = connect_async("ws://127.0.0.1:8080")
        .await
        .expect("Failed to connect");

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

    match msg {
        DesktopdMessage::ClientList { data } => {
            for item in data {
                tx_item.send(Arc::new(item)).unwrap()
            }
        }
        _ => {}
    }
}

fn main() {
    let (tx_item, rx_item): (SkimItemSender, SkimItemReceiver) = unbounded();

    task::block_on(run(tx_item.clone()));

    let options = SkimOptionsBuilder::default()
        .height(Some("50%"))
        .multi(true)
        .preview(Some("clients")) // preview should be specified to enable preview window
        .build()
        .unwrap();

    let selected_items = Skim::run_with(&options, Some(rx_item))
        .map(|out| out.selected_items)
        .unwrap_or_else(|| Vec::new());

    for item in selected_items.iter() {
        println!("{}", item.output());
    }
}
