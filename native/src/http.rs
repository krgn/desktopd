use crate::error::DesktopdError;
use crate::message::{CliRequest, DesktopdMessage};
use crate::state::{GlobalState, Tx};
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
    let (_state, tx) = req.state();
    tx.unbounded_send(DesktopdMessage::CliRequest(msg))
        .expect("Sending message failed");
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
