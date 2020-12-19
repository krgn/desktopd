use futures::{SinkExt, StreamExt};

use async_std::task;
use async_tungstenite::async_std::connect_async;
use async_tungstenite::tungstenite::protocol::Message;
use desktopd::browser::*;
use desktopd::message::*;
use skim::prelude::*;

type SinkHole = futures::stream::SplitSink<
    async_tungstenite::WebSocketStream<async_std::net::TcpStream>,
    async_tungstenite::tungstenite::Message,
>;

async fn run(tx_item: SkimItemSender) -> SinkHole {
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
    write
}

struct Foo {}

impl SkimItem for Foo {
    fn text(&self) -> Cow<str> {
        unimplemented!()
    }
}

#[async_std::main]
async fn main() {
    let (tx_item, rx_item): (SkimItemSender, SkimItemReceiver) = unbounded();

    let mut write_handle = task::block_on(run(tx_item.clone()));

    drop(tx_item);

    let options = SkimOptionsBuilder::default()
        .height(Some("50%"))
        .multi(false)
        .preview(None)
        .build()
        .unwrap();

    let selected_items = Skim::run_with(&options, Some(rx_item))
        .map(|out| out.selected_items)
        .unwrap_or_else(|| Vec::new());

    for item in selected_items.iter() {
        if let Some(client) = (**item).as_any().downcast_ref::<DesktopdClient>() {
            let command = match client {
                DesktopdClient::Window { data } => {
                    DesktopdMessage::CliRequest(CliRequest::FocusWindow { id: data.id })
                }
                DesktopdClient::Tab { data } => {
                    DesktopdMessage::CliRequest(CliRequest::FocusTab(BrowserTabRef {
                        tab_id: data.id,
                        window_id: data.window_id,
                    }))
                }
            };

            let msg = Message::Text(serde_json::to_string(&command).unwrap());

            write_handle
                .send(msg)
                .await
                .expect("could not send message");
        }
    }
}
