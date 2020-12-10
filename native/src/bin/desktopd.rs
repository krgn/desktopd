#![feature(async_closure)]
use async_std::task;
use desktopd::state::*;
use desktopd::sway;
use desktopd::tmux;
use desktopd::websocket;
use log::info;
use std::io;
use std::sync::Mutex;

#[async_std::main]
async fn main() -> io::Result<()> {
    let _ = env_logger::try_init();

    let state = GlobalState::new(Mutex::new(State::new()));

    let tmux_state = state.clone();
    task::spawn(async {
        info!("tmux events starting");
        tmux::connection::run(tmux_state)
            .await
            .expect("Coult not start tmux)");
    });

    let ws_state = state.clone();
    task::spawn(async {
        info!("ws server starting");
        websocket::run(ws_state).await.expect("Yes.");
        info!("but what now?");
    });

    sway::connection::run(state).await
}
