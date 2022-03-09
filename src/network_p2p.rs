use std::net::{SocketAddr, IpAddr};
use std::time::Duration;
use std::sync::mpsc::{Sender, Receiver};
use tokio::{spawn, select};
use tokio::time::sleep;
use tokio::task::spawn_blocking;
use tokio::sync::mpsc::{channel, Sender as TSender, Receiver as TReceiver};
use tokio::runtime::Handle;
// use tokio::sync::watch::Receiver as WReceiver;
use tokio::sync::watch::Sender as WSender;
use qp2p::{Config, Endpoint, ConnectionIncoming, Connection};
use crate::network::{FromNet, ToNet, show_error};


// struct PeerKnown {
//     address: SocketAddr,
//     last_seen: Instant,
// }

pub async fn task_p2p(from_app: Receiver<ToNet>, to_app: Sender<FromNet>,
        send_port: WSender<Option<u16>>, receive_peer: TReceiver<SocketAddr>) {
    let (to_output, connecting) = channel(1);

    let a = to_app.clone();
    let handle = spawn(task_send(from_app, a, connecting));

    task_receive(to_app, to_output, send_port).await;
    handle.await.expect("task panicked");
}

async fn task_receive(mut to_app: Sender<FromNet>, to_output: TSender<Connection>,
        send_port: WSender<Option<u16>>) {
    loop {
        // TODO: maybe wait until a remote peer is discovered before building the endpoint

        if let Err(_) = send_port.send(None) {
            return;
        }
        
        let ep = Endpoint::new_peer(
            SocketAddr::from(([0, 0, 0, 0], 0)),
            &[], // TODO: can put broadcast peer list here, maybe
            Config {
                idle_timeout: Duration::from_secs(60 * 60).into(), // 1 hour idle timeout.
                ..Default::default()
            },
        ).await;

        let (node, mut incoming_conns, _contact) = match ep {
            Ok(a) => a,
            Err(error) => {
                if !show_error(&mut to_app, format!("error: {:?}", error)) {
                    return;
                }

                sleep(Duration::from_secs(5)).await;
                continue;
            }
        };

        let port = node.public_addr().port();
        if let Err(_) = send_port.send(Some(port)) {
            return;
        }

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

async fn task_send(from_app: Receiver<ToNet>, mut to_app: Sender<FromNet>,
        mut connecting: TReceiver<Connection>) {

    let mut commands = pull_commands(from_app);
    let mut connections = Vec::<Connection>::new();
    loop {
        select! {
            command = commands.recv() => {
                let command = if let Some(a) = command {
                    a
                } else {
                    break;
                };
                on_command(&mut to_app, &connections, command).await;
            }
            connection = connecting.recv() => {
                let connection = if let Some(a) = connection {
                    a
                } else {
                    break;
                };
                on_connection(&mut connections, connection);
            }
        }
    }
}

fn pull_commands(from_app: Receiver<ToNet>) -> TReceiver<ToNet> {
    // TODO: Can the tokio channel be directly used from the main thread?

    let (to_outer, commands) = channel(1);

    spawn_blocking(move || {
        while let Ok(command) = from_app.recv() {
            let f = to_outer.send(command);

            if let Err(_) = Handle::current().block_on(f) {
                return;
            }
        }
    });

    commands
}

async fn on_command(to_app: &mut Sender<FromNet>, connections: &[Connection], command: ToNet) {        
    match command {
        ToNet::Send {message_id, address, content} => {
            let found = connections
                .iter().find(|r| r.remote_address().ip() == address);
            if let Some(dest) = found {
                if let Err(error) = dest.send(content.into()).await {
                    if !show_error(to_app, format!("error: {:?}", error)) {
                        return;
                    }
                    if let Err(_) = to_app.send(FromNet::SendFailed(message_id)) {
                        return;
                    }
                } else {
                    if let Err(_) = to_app.send(FromNet::SendArrived(message_id)) {
                        return;
                    }
                }
            } else {

                // let (conn, mut incoming) = node.connect_to(&peer).await?;
                // conn.send(msg.clone()).await?;

                if !show_error(to_app,
                        format!("error: no connection to {}", address)) {
                    return;
                }
                if let Err(_) = to_app.send(FromNet::SendFailed(message_id)) {
                    return;
                }
            }
        }
    }
}

fn on_connection(connections: &mut Vec<Connection>, connection: Connection) {
    let ip = connection.remote_address().ip();

    if let Some(index) = connections
            .iter().position(|r| r.remote_address().ip() == ip) {
        // TODO: will this ever happen or does qp2p keep track?
        connections[index].close(None);
        connections[index] = connection;
    } else {
        connections.push(connection);
    }
}
