use std::net::{SocketAddr, IpAddr};
use std::time::Duration;
use std::sync::mpsc::{Sender, Receiver};
use tokio::spawn;
use tokio::time::sleep;
use qp2p::{Config, Endpoint, ConnectionIncoming};
use crate::network::{FromNet, ToNet, show_error};

pub async fn task_receive(mut to_app: Sender<FromNet>) {
    loop {
        // TODO: maybe wait until a remote peer is discovered before building the endpoint
        
        let ep = Endpoint::new_peer(
            SocketAddr::from(([0, 0, 0, 0], 0)),
            &[], // TODO: can put broadcast peer list here, maybe
            Config {
                idle_timeout: Duration::from_secs(60 * 60).into(), // 1 hour idle timeout.
                ..Default::default()
            },
        ).await;

        let (_node, mut incoming_conns, _contact) = match ep {
            Ok(a) => a,
            Err(error) => {
                if !show_error(&mut to_app, format!("error: {:?}", error)) {
                    return;
                }

                sleep(Duration::from_secs(5)).await;
                continue;
            }
        };

        while let Some((connection, incoming_messages)) = incoming_conns.next().await {
            let source = connection.remote_address().ip();

            spawn(task_receive_one(to_app.clone(), source, incoming_messages));

            // TODO: put connection into message channel or some such
        }
    }
}

pub async fn task_receive_one(
        to_app: Sender<FromNet>, source: IpAddr, mut incoming: ConnectionIncoming) {
    while let Some(bytes) = incoming.next().await.unwrap() {
        if let Err(_) = to_app.send(FromNet::ShowMessage {
            source,
            content: String::from_utf8_lossy(&bytes).into_owned(),
        }) {
            return;
        }
    }
}

pub async fn task_blocking_send(to_app: Sender<FromNet>, from_app: Receiver<ToNet>) {
    while let Ok(command) = from_app.recv() {
        match command {
            ToNet::Send {message_id, address, content} => {
                // address: IpAddr,

                // let connection = ??? <-- address
                // if let Err(error) = connection.send(Bytes::from(content)).await {
                //     if !show_error(&mut to_app, format!("error: {:?}", error)) {
                //         return;
                //     }
                //     if let Err(_) = to_app.send(FromNet::SendFailed(message_id)) {
                //         return;
                //     }
                // } else {
                //     if let Err(_) = to_app.send(FromNet::SendArrived(message_id)) {
                //         return;
                //     }
                // }
            }
        }
    }
}
