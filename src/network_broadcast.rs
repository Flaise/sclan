use std::str::from_utf8;
use std::cmp::min;
use std::net::{IpAddr, SocketAddr};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::Duration;
use std::io::{Error as IOError, Result as IOResult, ErrorKind};
use tokio::net::UdpSocket;
use tokio::time::sleep;
use tokio::select;
use tokio::sync::watch::Receiver as WReceiver;
use tokio::sync::mpsc::Sender as TSender;
use gethostname::gethostname;
use crate::network::{show_status, show_error, FromNet};

const PORT: u16 = 31331;

pub async fn task_ping(mut to_app: Sender<FromNet>, wport: WReceiver<Option<u16>>,
        to_p2p: TSender<SocketAddr>) {
    loop {
        let socket = match make_socket().await {
            Err(error) => {
                if error.kind() == ErrorKind::AddrInUse {
                    if !show_error(&mut to_app, "error: address already in use") {
                        return;
                    }
                } else {
                    if !show_error(&mut to_app, format!("error: {:?}", error)) {
                        return;
                    }
                }

                sleep(Duration::from_secs(5)).await;
                continue;
            }
            Ok(a) => a,
        };
        let socket = Arc::new(socket);

        if !show_status(&mut to_app, "connected") {
            return;
        }
        show_local_ip(&mut to_app);

        let pout = task_ping_out(socket.clone(), to_app.clone(), wport.clone());
        let pin = task_ping_in(socket, to_app.clone(), to_p2p.clone());

        let done = select! {
            a = pout => a,
            a = pin => a,
        };
        match done {
            PingDone::Exiting => return,
            PingDone::IO(error) => {
                if !show_error(&mut to_app, format!("error: {:?}", error)) {
                    return;
                }
            }
        }

        if let Err(_) = to_app.send(FromNet::ShowLocalAddress("".into())) {
            return;
        }

        sleep(Duration::from_secs(5)).await;
    }
}

enum PingDone {
    Exiting,
    IO(IOError),
}

async fn task_ping_in(socket: Arc<UdpSocket>, mut to_app: Sender<FromNet>,
        to_p2p: TSender<SocketAddr>) -> PingDone {
    let to_app = &mut to_app;
    let mut buf = [0; 2048];
    loop {
        let (count, source) = match socket.recv_from(&mut buf).await {
            Ok(a) => a,
            Err(error) => return PingDone::IO(error),
        };
        let ip = source.ip();
        if ip == IpAddr::from([127, 0, 0, 1]) {
            continue;
        }

        let message = &buf[..count];

        let (name, port) = if let Some(a) = parse_ping(message) {
            a
        } else {
            if !show_status(to_app, format!("invalid ping from {:?}", source)) {
                return PingDone::Exiting;
            }
            continue;
        };

        let peer = FromNet::Peer {
            name: name.to_string(),
            address: ip,
        };
        if let Err(_) = to_app.send(peer) {
            return PingDone::Exiting;
        }

        let peer_addr = SocketAddr::from((ip, port));
        if let Err(_) = to_p2p.send(peer_addr).await {
            return PingDone::Exiting;
        }
    }
}

// /// Returns None if the other end of the watch channel is dropped.
// async fn extract_some<T>(watching: WReceiver<Option<T>>) -> Option<T> {
//     loop {
//         if let Some(val) = watching.borrow() {
//             return Some(val);
//         }
//         if let Err(_) = watching.changed().await {
//             return None;
//         }
//     }
// }

async fn task_ping_out(socket: Arc<UdpSocket>, mut to_app: Sender<FromNet>,
        mut wport: WReceiver<Option<u16>>) -> PingDone {
    loop {
        let name = gethostname().into_string().unwrap_or("???".into());

        let port = loop {
            let val = *wport.borrow();
            match val {
                Some(a) => break a,
                None => {
                    if let Err(_) = wport.changed().await {
                        return PingDone::Exiting;
                    }
                }
            }
        };

        if let Err(error) = send_ping(&socket, &name, port).await {
            if !show_error(&mut to_app, format!("ping error: {:?}", error)) {
                return PingDone::Exiting;
            }
            return PingDone::IO(error);
        }

        sleep(Duration::from_secs(4)).await;
    }
}

async fn make_socket() -> IOResult<UdpSocket> {
    let socket = UdpSocket::bind(("0.0.0.0", PORT)).await?;
    socket.set_broadcast(true)?;
    Ok(socket)
}

async fn send_ping(socket: &Arc<UdpSocket>, local_name: &str, port: u16) -> IOResult<()> {
    let len = min(local_name.len(), u8::max_value() as usize);
    let mut message = vec![len as u8];
    message.extend_from_slice(&local_name.as_bytes()[0..len]);

    message.push(0);

    message.extend_from_slice(&port.to_be_bytes());

    socket.send_to(&message, ("255.255.255.255", PORT)).await?;
    Ok(())
}

fn parse_ping(message: &[u8]) -> Option<(&str, u16)> {
    let len = *message.get(0)?;
    let name_bytes = message.get(1..1 + len as usize)?;
    let name = from_utf8(name_bytes).ok()?;

    if message.get(1 + len as usize) != Some(&0) {
        // use zero termination just to make it easier to catch malformed packets
        return None;
    }

    let port_index = 2 + len as usize;
    let port_bytes = message.get(port_index..port_index + 2)?;

    let port = u16::from_be_bytes(port_bytes.try_into().unwrap());
    if port == 0 {
        return None;
    }

    Some((name, port))
}

fn local_ip() -> Option<IpAddr> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    Some(socket.local_addr().ok()?.ip())
}

fn show_local_ip(to_app: &mut Sender<FromNet>) {
    let addr = local_ip()
        .map(|a| a.to_string())
        .unwrap_or("".to_string());
    let _ignore = to_app.send(FromNet::ShowLocalAddress(addr));
}
