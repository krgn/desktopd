#![feature(async_closure)]
use async_std::task;
use desktopd::state::*;
use desktopd::sway;
use desktopd::websocket;
use futures::channel::mpsc::unbounded;
use log::info;
use std::io;
use std::sync::Mutex;

#[async_std::main]
async fn main() -> io::Result<()> {
    let _ = env_logger::try_init();

    let (sway_tx, sway_rx) = unbounded();
    let state = GlobalState::new(Mutex::new(State::new()));

    let ws_state = state.clone();
    let sway_tx_handle = sway_tx.clone();
    task::spawn(async {
        info!("ws server starting");
        websocket::run(ws_state, sway_tx_handle)
            .await
            .expect("Websocket server failed");
    });

    sway::connection::run(state, sway_tx, sway_rx).await
}
