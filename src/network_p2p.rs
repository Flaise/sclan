use std::net::{SocketAddr, IpAddr};
use std::time::Duration;
use std::sync::mpsc::{Sender, Receiver};
use tokio::spawn;
use tokio::time::sleep;
use tokio::task::{spawn_blocking, JoinHandle};
// use tokio::sync::mpsc::channel;
use qp2p::{Config, Endpoint, ConnectionIncoming};
use crate::network::{FromNet, ToNet, show_error};

pub async fn task_p2p(from_app: Receiver<ToNet>, to_app: Sender<FromNet>) {
    let a = to_app.clone();
    let handle = start_task_send(from_app, a);

    task_receive(to_app).await;
    handle.await.expect("task panicked");
}

async fn task_receive(mut to_app: Sender<FromNet>) {
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

async fn task_receive_one(
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

fn start_task_send(from_app: Receiver<ToNet>, to_app: Sender<FromNet>) -> JoinHandle<()> {
    // let (tx, mut rx) = channel(32);

    spawn_blocking(move || {
        while let Ok(command) = from_app.recv() {
            let to_app = to_app.clone();
            spawn(async move {
                match command {
                    ToNet::Send {message_id, address, content} => {
                        if let Err(_) = to_app.send(FromNet::SendFailed(message_id)) {
                            // not necessary to bubble up exit signal because from_app will be
                            // dropped too
                            return;
                        }
                    }
                }
            });
        }
    })
    // task.await.expect("task panicked");

    // loop {
    //     // let from_app = from_app.clone();
    //     // let r = .await.expect("task panicked");

    //     // let command = match r {
    //     //     Err(RecvError) => return,
    //     //     Ok(a) => a,
    //     // };
    //     match command {
    //         ToNet::Send {message_id, address, content} => {
    //             // address: IpAddr,

    //             // let connection = ??? <-- address
    //             // if let Err(error) = connection.send(Bytes::from(content)).await {
    //             //     if !show_error(&mut to_app, format!("error: {:?}", error)) {
    //             //         return;
    //             //     }
    //             //     if let Err(_) = to_app.send(FromNet::SendFailed(message_id)) {
    //             //         return;
    //             //     }
    //             // } else {
    //             //     if let Err(_) = to_app.send(FromNet::SendArrived(message_id)) {
    //             //         return;
    //             //     }
    //             // }
    //         }
    //     }
    // }
}
