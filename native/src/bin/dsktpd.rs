use futures::{SinkExt, StreamExt};

use async_std::task;
use async_tungstenite::async_std::connect_async;
use async_tungstenite::tungstenite::protocol::Message;
use desktopd::browser::*;
use desktopd::message::*;
use notify_rust::Notification;
use skim::prelude::*;
use tabular::{Row, Table};
use url::Url;

const WIDTH: usize = 80;

struct Wrapper {
    client: DesktopdClient,
    line: String,
}

impl SkimItem for Wrapper {
    fn text(&self) -> Cow<str> {
        Cow::Borrowed(&self.line)
    }
}

type SinkHole = futures::stream::SplitSink<
    async_tungstenite::WebSocketStream<async_std::net::TcpStream>,
    async_tungstenite::tungstenite::Message,
>;

async fn run(tx_item: SkimItemSender) -> SinkHole {
    let width = std::env::var("DSKTPD_CLIENT_WIDTH")
        .map(|w| usize::from_str_radix(&w, 10).unwrap_or(WIDTH))
        .unwrap_or(WIDTH);

    let (ws_stream, _) = connect_async("ws://127.0.0.1:8080")
        .await
        .unwrap_or_else(|e| {
            Notification::new()
                .summary("desktopd")
                .body("Error: could not connect to daemon.")
                .show()
                .expect("Could not show notification");
            panic!("Fatal error: {}", e)
        });

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

    if let DesktopdMessage::ClientList { data } = msg {
        let mut table = Table::new("{:<} {:<} {:<}");

        for item in &data {
            let row = match item {
                DesktopdClient::Window { data } => {
                    let id_or_class = if data.app_id.len() == 0 {
                        data.class.to_owned()
                    } else {
                        data.app_id.to_owned()
                    };

                    Row::new()
                        .with_cell("app")
                        .with_cell(&id_or_class)
                        .with_cell(&data.name)
                }

                DesktopdClient::Tab { data } => {
                    let formatted_title = if data.title.len() > width {
                        format!("{}...", &data.title[0..width])
                    } else {
                        data.title.clone()
                    };

                    let mut url = Url::parse(&data.url).unwrap();
                    url.set_path("");
                    url.set_query(None);
                    url.set_fragment(None);

                    Row::new()
                        .with_cell("tab")
                        .with_cell(formatted_title)
                        .with_cell(url.to_string())
                }
            };
            table.add_row(row);
        }

        table
            .to_string()
            .split("\n")
            .zip(data)
            .map(|(line, client)| Wrapper {
                client,
                line: line.to_owned(),
            })
            .for_each(|wrap| tx_item.send(Arc::new(wrap)).unwrap())
    }

    write
}

#[async_std::main]
async fn main() {
    let (tx_item, rx_item): (SkimItemSender, SkimItemReceiver) = unbounded();

    let mut write_handle = task::block_on(run(tx_item.clone()));

    drop(tx_item);

    let options = SkimOptionsBuilder::default()
        .multi(false)
        .preview(None)
        .build()
        .unwrap();

    let selected_items = Skim::run_with(&options, Some(rx_item))
        .map(|out| match out.final_key {
            Key::ESC => Vec::new(),
            _ => out.selected_items,
        })
        .unwrap_or_else(|| Vec::new());

    for item in selected_items.iter() {
        if let Some(wrapper) = (**item).as_any().downcast_ref::<Wrapper>() {
            use CliRequest as Req;
            use DesktopdClient as DC;
            use DesktopdMessage as DM;
            let command = match &wrapper.client {
                DC::Window { data } => DM::CliRequest(Req::FocusWindow { id: data.id }),

                DC::Tab { data } => DM::CliRequest(Req::FocusTab(BrowserTabRef {
                    tab_id: data.id,
                    window_id: data.window_id,
                })),
            };

            let msg = Message::Text(serde_json::to_string(&command).unwrap());

            write_handle
                .send(msg)
                .await
                .expect("could not send message");
        }
    }
}
