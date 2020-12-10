use crate::state::*;
use async_process::{Command, Stdio};
use futures::prelude::*;
use futures::{future, io::BufReader};
use log::info;
use std::io;

pub async fn run(_state: GlobalState) -> io::Result<()> {
    let mut tmux = Command::new("tmux")
        .arg("-C")
        .arg("attach")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    // TODO: wire it up so that we can send comands to tmux
    //
    // let (tx, rx) = unbounded::<String>();
    // let input = BufWriter::new(tmux.stdin.take().unwrap());
    let lines = BufReader::new(tmux.stdout.take().unwrap()).lines();

    let handle = lines.try_for_each(|line: String| {
        if !line.starts_with("%output") {
            info!("tmux: {}", line);
        }
        future::ok(())
    });

    handle.await
}
