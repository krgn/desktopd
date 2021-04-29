use crate::error::DesktopdError;
use crate::message::{CliRequest, DesktopdMessage};
use crate::state::{GlobalState, Tx};
use log::error;
use log::info;
use std::env;
use tide::Request;

pub async fn get_clients(req: Request<(GlobalState, Tx)>) -> tide::Result {
    let (state, _tx) = req.state();
    let clients = { state.lock().unwrap().clients() };
    let json = serde_json::to_string(&clients).expect("Could not serialzie client list");
    Ok(json.into())
}

pub async fn post_command(mut req: Request<(GlobalState, Tx)>) -> tide::Result {
    let msg: CliRequest = req.body_json().await?;
    let (state, tx) = req.state();

    use CliRequest::*;
    match msg {
        FocusWindow { .. } => tx
            .unbounded_send(DesktopdMessage::CliRequest(msg.clone()))
            .expect("Sending message failed"),
        FocusTab { .. } => {
            for (peer_addr, peer) in state.lock().unwrap().get_browser_connections() {
                match peer.unbounded_send(DesktopdMessage::CliRequest(msg.clone())) {
                    Ok(_) => info!("Successfully sent focus-tab message to browsers"),
                    Err(e) => error!("Could not send message to browser {}: {}", peer_addr, e),
                }
            }
        }
    }

    Ok("Ok".into())
}

pub async fn run(state: GlobalState, sway_tx: Tx) -> Result<(), DesktopdError> {
    let addr = env::args().nth(2).unwrap_or("127.0.0.1:8081".to_owned());
    let mut app = tide::with_state((state, sway_tx));

    app.at("/clients").get(get_clients);
    app.at("/cmd").post(post_command);

    app.listen(addr).await?;
    Ok(())
}
