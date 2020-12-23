use crate::browser::*;
use crate::message::*;
use crate::state::GlobalState;
use crate::state::{Rx, Tx};
use crate::sway::types::SwayWindow;
use async_i3ipc::{
    event::{Event, Subscribe, WindowChange, WindowData},
    reply::{FullscreenMode, Node, NodeLayout},
    I3,
};
use async_std::task;
use futures::prelude::*;
use futures::{future, pin_mut};
use log::info;
use std::io;
use std::time::Duration;

// ░█▀█░█░█░█▀▄░█░░░▀█▀░█▀▀
// ░█▀▀░█░█░█▀▄░█░░░░█░░█░░
// ░▀░░░▀▀▀░▀▀░░▀▀▀░▀▀▀░▀▀▀

pub async fn run(state: GlobalState, tx: Tx, rx: Rx) -> io::Result<()> {
    initialize_state(state.clone()).await?;

    let listener_state = state.clone();
    let listener = task::spawn(async move {
        sway_event_process(listener_state).await;
    });

    let commando = task::spawn(async move {
        sway_command_process(state, tx, rx).await;
    });

    pin_mut!(listener, commando);
    future::select(listener, commando).await;

    Ok(())
}

// ░█▀█░█▀▄░▀█▀░█░█░█▀█░▀█▀░█▀▀
// ░█▀▀░█▀▄░░█░░▀▄▀░█▀█░░█░░█▀▀
// ░▀░░░▀░▀░▀▀▀░░▀░░▀░▀░░▀░░▀▀▀

async fn initialize_state(state: GlobalState) -> io::Result<()> {
    let mut i3 = I3::connect().await?;
    let tree = i3.get_tree().await?;
    let windows = SwayWindow::collect_windows(&tree);

    for win in windows {
        state.lock().unwrap().add_window(win);
    }

    Ok(())
}

async fn sway_command_process(state: GlobalState, tx: Tx, rx: Rx) {
    let mut receiver = rx;
    let mut i3 = I3::connect().await.expect("Failed connecting to sway");

    while let Some(msg) = receiver.next().await {
        handle_incoming_message(&mut i3, state.clone(), tx.clone(), msg).await;
    }
}

async fn sway_event_process(state: GlobalState) {
    let mut listener_sway = I3::connect().await.expect("Cannot connect to sway");

    let _resp = listener_sway
        .subscribe([Subscribe::Window])
        .await
        .expect("Subscription failed");

    let mut listener = listener_sway.listen();

    let mut sway = I3::connect().await.expect("Cannot connect to sway");

    use Event::*;
    while let Ok(event) = listener.next().await {
        match event {
            Window(data) => handle_window_event(&mut sway, state.clone(), *data).await,
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

async fn handle_incoming_message(i3: &mut I3, state: GlobalState, tx: Tx, msg: DesktopdMessage) {
    use DesktopdMessage::*;
    match msg {
        CliRequest(req) => handle_cli_request(i3, req).await,
        BrowserMessage { data } => {
            handle_browser_response(i3, state.clone(), tx.clone(), data).await
        }
        _ => (),
    }
}

async fn handle_cli_request(i3: &mut I3, req: CliRequest) {
    if let CliRequest::FocusWindow { id } = req {
        i3.run_command(format!("[con_id={}] focus", id))
            .await
            .expect("Error running command");
    }
}

/// Handling browser response messages
///
/// When a tab is focused, the browser responds with an activated message. We us this to try and
/// focus the browser window via sway, since its Window title must now have changed to the tab
/// title.
///
async fn handle_browser_response(i3: &mut I3, state: GlobalState, tx: Tx, resp: BrowserResponse) {
    use BrowserResponse::*;
    use DesktopdMessage::*;
    match resp {
        Activated(tab_ref) => {
            let browser = {
                let current = state.lock().unwrap();
                if let Some(tab) = current.find_tab(&tab_ref) {
                    current
                        .get_browser_windows()
                        .iter()
                        .filter(|win| {
                            win.name.contains(&tab.title)
                                || (tab.url == "about:blank" && win.name == "Mozilla Firefox")
                        })
                        .map(|win| *win)
                        .collect::<Vec<&SwayWindow>>()
                        .first()
                        .map(|browser| browser.id)
                } else {
                    None
                }
            };
            if let Some(id) = browser {
                i3.run_command(format!("[con_id={}] focus", id))
                    .await
                    .expect("Error running command");
            } else {
                let back_channel = tx.clone();
                let _handle = task::spawn(async move {
                    let retry = BrowserMessage {
                        data: BrowserResponse::Activated(tab_ref),
                    };
                    task::sleep(Duration::from_millis(1)).await;
                    back_channel.unbounded_send(retry).expect("Sending failed");
                });
            }
        }
        _ => (),
    }
}

fn mark_focused(state: GlobalState, data: &WindowData) {
    let mut state = state.lock().unwrap();
    let focused = state.remove_focused();
    for win in focused {
        state.add_window(SwayWindow {
            focused: false,
            ..win
        })
    }
    let windows = SwayWindow::collect_windows(&data.container);
    for win in windows {
        state.add_window(win)
    }
}

fn find_parent<'a>(node: &'a Node, tree: &'a Node) -> Option<&'a Node> {
    let mut it_matches = false;

    for child in &tree.nodes {
        if it_matches {
            break;
        };
        it_matches = child.id == node.id;
    }

    for child in &tree.floating_nodes {
        if it_matches {
            break;
        };
        it_matches = child.id == node.id;
    }

    if it_matches {
        return Some(tree);
    }

    for child in &tree.nodes {
        if let Some(parent) = find_parent(&node, &child) {
            return Some(parent);
        }
    }

    for child in &tree.floating_nodes {
        if let Some(parent) = find_parent(&node, &child) {
            return Some(parent);
        }
    }

    None
}

async fn maybe_split_container(i3: &mut I3, data: &WindowData) {
    let con = &data.container;

    // if Some("desktopd-launcher".to_owned()) == con.app_id {
    //     return;
    // }

    let tree = i3.get_tree().await.expect("I want my tree");
    let parent = find_parent(&con, &tree).expect("Container must have a parent");
    let is_not_floating = con.floating.is_none();
    let is_fullscreen = con.fullscreen_mode != FullscreenMode::None;
    let is_stacked = parent.layout == NodeLayout::Stacked;
    let is_tabbed = parent.layout == NodeLayout::Tabbed;
    let should_split = is_not_floating && !is_fullscreen && !is_stacked && !is_tabbed;
    if should_split {
        let new_layout = if con.rect.height > con.rect.width {
            "splitv"
        } else {
            "splith"
        };
        i3.run_command(new_layout).await.unwrap();
    }
}

async fn handle_window_event(i3: &mut I3, state: GlobalState, data: WindowData) {
    info!("handleing {:#?} event", data.change);
    match data.change {
        WindowChange::Close => {
            info!("removing window: {:#?}", data.container.id);
            state.lock().unwrap().remove_window(&data.container.id)
        }

        WindowChange::Focus => {
            mark_focused(state.clone(), &data);
            maybe_split_container(i3, &data).await;
        }

        _ => {
            for win in SwayWindow::collect_windows(&data.container) {
                state.lock().unwrap().add_window(win)
            }
        }
    }
}
