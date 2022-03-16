use std::net::{SocketAddr, IpAddr};
use std::time::Duration;
use std::sync::mpsc::{Sender, Receiver};
use tokio::{spawn, select};
use tokio::time::{sleep, interval, MissedTickBehavior, Instant};
use tokio::task::spawn_blocking;
use tokio::sync::mpsc::{channel, Receiver as TReceiver, Sender as TSender};
use tokio::runtime::Handle;
use tokio::sync::watch::Sender as WSender;
use qp2p::{Config, Endpoint, ConnectionIncoming, Connection};
use crate::network::{FromNet, ToNet, show_error};
use crate::log::ToLog;

const PEER_IDLE_TIME: Duration = Duration::from_secs(18);

struct PeerKnown {
    name: String,
    address: SocketAddr,
    last_seen: Instant,
}

pub async fn task_p2p(from_app: Receiver<ToNet>, mut to_app: Sender<FromNet>,
        mut to_log: TSender<ToLog>,
        send_port: WSender<Option<u16>>, mut receive_peer: TReceiver<(SocketAddr, String)>) {

    let mut peers_known = Vec::<PeerKnown>::new();
    let mut commands = pull_commands(from_app);
    'restart: loop {
        // TODO: maybe wait until a remote peer is discovered before building the endpoint

        if let Err(_) = send_port.send(None) {
            return;
        }
        
        let ep = Endpoint::new_peer(
            SocketAddr::from(([0, 0, 0, 0], 0)),
            &peers_known
                .iter().map(|r| r.address).collect::<Vec<_>>(),
            Config {
                idle_timeout: Some(Duration::from_secs(60 * 5)),
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

        let mut interval = interval(Duration::from_secs(5));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        let mut connections = Vec::<Connection>::new();
        loop {
            select! {
                now = interval.tick() => {
                    cull_peers(&mut to_app, &mut peers_known, now);
                }

                command = commands.recv() => {
                    let command = if let Some(a) = command {
                        a
                    } else {
                        return;
                    };

                    on_command(&mut to_app, &mut to_log, &node, &mut connections, &peers_known,
                        command).await;
                }

                peer = receive_peer.recv() => {
                    let (address, name) = if let Some(a) = peer {
                        a
                    } else {
                        return;
                    };

                    on_peer(&mut peers_known, address, name);
                }

                arrival = incoming_conns.next() => {
                    let (connection, incoming_messages) = if let Some(a) = arrival {
                        a
                    } else {
                        sleep(Duration::from_secs(5)).await;
                        continue 'restart;
                    };

                    on_connection(to_app.clone(), to_log.clone(),
                        &mut connections, &peers_known, connection, incoming_messages);
                }
            }
        }
    }
}

async fn task_receive_one(to_app: Sender<FromNet>, to_log: TSender<ToLog>,
        source: IpAddr, name: String, mut incoming: ConnectionIncoming) {
    while let Ok(obytes) = incoming.next().await {
        let bytes = if let Some(a) = obytes {
            a
        } else {
            return;
        };

        let content = String::from_utf8_lossy(&bytes).into_owned();
        
        if let Err(_) = to_log.send(ToLog::LogMessage(
            format!("\nfrom [{}] [{}] {}", name, source.to_string(), content.clone())
        )).await {
            return;
        }
        if let Err(_) = to_app.send(FromNet::ShowMessage {source, content}) {
            return;
        }
    }
}

fn pull_commands(from_app: Receiver<ToNet>) -> TReceiver<ToNet> {
    // TODO: Can a tokio channel be directly used from the main thread?

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

async fn send_message(connections: &mut Vec<Connection>,
        address: IpAddr, content: String) -> Result<(), String> {

    let found = connections
        .iter().find(|r| r.remote_address().ip() == address);
    let dest = found.ok_or(format!("no connection to {}", address))?;
    dest.send(content.into()).await
        .map_err(|a| a.to_string())
}

/// Returns Result<name of peer, description of failure>
async fn send_twice(to_app: &mut Sender<FromNet>, to_log: &mut TSender<ToLog>,
        node: &Endpoint, connections: &mut Vec<Connection>, peers: &[PeerKnown],
        address: IpAddr, content: String) -> Result<String, String> {
    let found = peers.iter().find(|r| r.address.ip() == address);
    let peer = found.ok_or(format!("no connection to {}", address))?;
    
    if let Ok(_) = send_message(connections, address, content.clone()).await {
        return Ok(peer.name.clone());
    }
    
    let (conn, incoming_messages) = node.connect_to(&peer.address).await
        .map_err(|a| a.to_string())?;
    on_connection(to_app.clone(), to_log.clone(), connections, peers, conn, incoming_messages);
    
    send_message(connections, address, content.clone()).await
        .map_err(|a| a.to_string())?;
        
    Ok(peer.name.clone())
}

async fn on_command(to_app: &mut Sender<FromNet>, to_log: &mut TSender<ToLog>, node: &Endpoint,
        connections: &mut Vec<Connection>, peers: &[PeerKnown], command: ToNet) {
    match command {
        ToNet::Send {message_id, address, content} => {
            match send_twice(to_app, to_log,
                    node, connections, peers, address, content.clone()).await {
                Ok(name) => {
                    if let Err(_) = to_app.send(FromNet::SendArrived(message_id)) {
                        return;
                    }
                    if let Err(_) = to_log.send(ToLog::LogMessage(
                        format!("\nto [{}] [{}] {}", name, address.to_string(), content)
                    )).await {
                        return;
                    }
                }
                Err(error) => {
                    if !show_error(to_app, format!("error: {:?}", error)) {
                        return;
                    }
                    if let Err(_) = to_app.send(FromNet::SendFailed(message_id)) {
                        return;
                    }
                }
            }
        }
        ToNet::LogStart => {
            if let Err(_) = to_log.send(ToLog::LogStart).await {
                return;
            }
        }
    }
}

fn cull_peers(to_app: &mut Sender<FromNet>, peers_known: &mut Vec<PeerKnown>, now: Instant) {
    peers_known.retain(|peer| {
        if now.duration_since(peer.last_seen) < PEER_IDLE_TIME {
            true
        } else {
            let _ = to_app.send(FromNet::Peerbgone(peer.address.ip()));
            false
        }
    });
}

fn on_peer(peers_known: &mut Vec<PeerKnown>, address: SocketAddr, name: String) {
    let ip = address.ip();

    if let Some(index) = peers_known
            .iter().position(|r| r.address.ip() == ip) {
        let peer = &mut peers_known[index];
        peer.name.clear();
        peer.name.push_str(&name);
        peer.last_seen = Instant::now();
    } else {
        peers_known.push(PeerKnown {
            name,
            address,
            last_seen: Instant::now(),
        });
    }
}

fn on_connection(to_app: Sender<FromNet>, to_log: TSender<ToLog>,
        connections: &mut Vec<Connection>, peers: &[PeerKnown],
        connection: Connection, incoming_messages: ConnectionIncoming) {
    let ip = connection.remote_address().ip();

    if let Some(index) = connections
            .iter().position(|r| r.remote_address().ip() == ip) {
        if connection.remote_address() != connections[index].remote_address() {
            connections[index].close(Some("connected on new port".into()));
        }
        connections[index] = connection;
    } else {
        connections.push(connection);
    }
    
    let found = peers.iter().find(|r| r.address.ip() == ip);
    let name = found.map(|a| a.name.clone()).unwrap_or(ip.to_string());

    spawn(task_receive_one(to_app, to_log, ip, name, incoming_messages));
}
