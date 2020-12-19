use crate::message::*;
use crate::state::GlobalState;
use crate::state::Rx;
use crate::sway::types::SwayWindow;
use async_i3ipc::{
    event::{Event, Subscribe, WindowChange, WindowData},
    I3,
};
use async_std::task;
use futures::prelude::*;
use futures::{future, pin_mut};
use log::info;
use std::io;

async fn initialize_state(state: GlobalState, i3: &mut I3) -> io::Result<()> {
    let tree = i3.get_tree().await?;
    let windows = SwayWindow::collect_windows(&tree);

    for win in windows {
        state.lock().unwrap().add_window(win);
    }

    Ok(())
}

async fn sway_command_process(rx: Rx) {
    let mut receiver = rx;
    let mut i3 = I3::connect().await.expect("Failed connecting to sway");

    loop {
        if let Some(msg) = receiver.next().await {
            match msg {
                DesktopdMessage::CliRequest(CliRequest::FocusWindow { id }) => {
                    i3.run_command(format!("[con_id={}] focus", id))
                        .await
                        .expect("Error running command");
                }
                _ => (),
            };
        } else {
            break;
        };
    }
}

fn handle_window_event(state: GlobalState, data: WindowData) {
    info!("handleing {:#?} event", data.change);
    let windows = SwayWindow::collect_windows(&data.container);
    match data.change {
        WindowChange::Close => {
            info!("removing window: {:#?}", data.container.id);
            state.lock().unwrap().remove_window(&data.container.id)
        }
        _ => {
            for win in windows {
                state.lock().unwrap().add_window(win)
            }
        }
    }
}

async fn sway_event_process(state: GlobalState, i3: I3) {
    let mut sway = i3;

    let _resp = sway
        .subscribe([Subscribe::Window])
        .await
        .expect("Subscription failed");

    let mut listener = sway.listen();

    use Event::*;
    while let Ok(event) = listener.next().await {
        match event {
            Window(data) => handle_window_event(state.clone(), *data),
            // Workspace(ev) => info!("workspace change event {:?}", ev),
            // Output(ev) => info!("output event {:?}", ev),
            // Mode(ev) => info!("mode event {:?}", ev),
            // BarConfig(ev) => info!("bar config update {:?}", ev),
            // Binding(ev) => info!("binding event {:?}", ev),
            // Shutdown(ev) => info!("shutdown event {:?}", ev),
            // Tick(ev) => info!("tick event {:?}", ev),
            _ => (),
        }
    }
}

pub async fn run(state: GlobalState, rx: Rx) -> io::Result<()> {
    let mut i3 = I3::connect().await?;

    initialize_state(state.clone(), &mut i3).await?;

    let listener = task::spawn(async move {
        sway_event_process(state, i3).await;
    });

    let commando = task::spawn(async move {
        sway_command_process(rx).await;
    });

    pin_mut!(listener, commando);
    future::select(listener, commando).await;

    Ok(())
}
