use crate::state::GlobalState;
use crate::sway::types::SwayWindow;
use async_i3ipc::{
    event::{Event, Subscribe},
    I3,
};
use log::info;
use std::io;

pub async fn run(state: GlobalState) -> io::Result<()> {
    let mut i3 = I3::connect().await?;
    let _resp = i3.subscribe([Subscribe::Window]).await?;

    let tree = i3.get_tree().await?;
    let windows = SwayWindow::collect_windows(&tree);
    for win in windows {
        state.lock().unwrap().add_window(win);
    }

    let mut listener = i3.listen();
    use Event::*;

    while let Ok(event) = listener.next().await {
        match event {
            Workspace(ev) => info!("workspace change event {:?}", ev),
            Window(ev) => info!("window event {:?}", ev),
            Output(ev) => info!("output event {:?}", ev),
            Mode(ev) => info!("mode event {:?}", ev),
            BarConfig(ev) => info!("bar config update {:?}", ev),
            Binding(ev) => info!("binding event {:?}", ev),
            Shutdown(ev) => info!("shutdown event {:?}", ev),
            Tick(ev) => info!("tick event {:?}", ev),
        }
    }

    Ok(())
}
