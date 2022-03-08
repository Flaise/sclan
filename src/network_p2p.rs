use std::net::{SocketAddr, IpAddr};
use std::time::Duration;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::{Arc, Mutex};
use tokio::spawn;
use tokio::time::sleep;
use tokio::task::spawn_blocking;
use tokio::sync::mpsc::{channel, Sender as TSender, Receiver as TReceiver};
use qp2p::{Config, Endpoint, ConnectionIncoming, Connection};
use crate::network::{FromNet, ToNet, show_error};

pub async fn task_p2p(from_app: Receiver<ToNet>, to_app: Sender<FromNet>) {
    let (to_output, connecting) = channel(1);

    let a = to_app.clone();
    let handle = spawn(task_send(from_app, a, connecting));

    task_receive(to_app, to_output).await;
    handle.await.expect("task panicked");
}

async fn task_receive(mut to_app: Sender<FromNet>, to_output: TSender<Connection>) {
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

            if let Err(_) = to_output.send(connection).await {
                // output task exited
                return;
            }
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

async fn task_send(from_app: Receiver<ToNet>, to_app: Sender<FromNet>,
        mut connecting: TReceiver<Connection>) {

    let connections = Arc::new(Mutex::new(Vec::<Connection>::new()));
    let conn2 = connections.clone();

    let task = spawn_blocking(move || {
        while let Ok(command) = from_app.recv() {
            let to_app = to_app.clone();
            let conn2 = conn2.clone();
            spawn(async move {
                match command {
                    ToNet::Send {message_id, address, content} => {


                        let connections = conn2.lock().expect("mutex poisoned");
                        let found = connections
                            .iter().find(|r| r.remote_address().ip() == address);
                        if let Some(dest) = found {
                            // if let Err(error) = dest.send(content.into()).await {
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
                        } else {
                            if let Err(_) = to_app.send(FromNet::SendFailed(message_id)) {
                                // not necessary to bubble up exit signal because from_app will be
                                // dropped too
                                return;
                            }
                        }
                    }
                }
            });
        }
    });

    while let Some(connection) = connecting.recv().await {
        let mut connections = connections.lock().expect("mutex poisoned");

        let ip = connection.remote_address().ip();

        if let Some(index) = connections.iter().position(|r| r.remote_address().ip() == ip) {
            // TODO: will this ever happen or does qp2p keep track?
            connections[index].close(None);
            connections[index] = connection;
        } else {
            connections.push(connection);
        }
    }

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
