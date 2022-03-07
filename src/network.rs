use std::net::{UdpSocket, IpAddr};
use std::cmp::min;
use std::time::{Duration, Instant};
use std::io::{ErrorKind, Result as IOResult};
use std::str::from_utf8;
use gethostname::gethostname;
use crate::data::{LANState, Peer, set_status, App, Message, now_fmt, MessageType};

const PORT: u16 = 31331;

pub fn network_update(app: &mut App) {
    if app.lan.local_name.len() == 0 {
        app.lan.local_name = gethostname().into_string().unwrap_or("???".into());
    }

    bind(app);
    ping(app);
}

fn local_ip() -> Option<IpAddr> {
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(_) => return None,
    };

    if let Err(_) = socket.connect("8.8.8.8:80") {
        return None;
    }

    match socket.local_addr() {
        Ok(addr) => return Some(addr.ip()),
        Err(_) => return None,
    };
}

fn update_local_ip(app: &mut App) {
    app.lan.local_addr = local_ip()
        .map(|a| a.to_string())
        .unwrap_or("???".to_string());
}

fn disconnect(app: &mut App) {
    app.lan.socket = None;
    app.lan.local_addr.clear();
}

fn check_interval(state: &mut LANState, interval: Duration) -> bool {
    let now = Instant::now();
    let ready = state.last_ping
        .map(|last| now.duration_since(last) >= interval)
        .unwrap_or(true);
    if ready {
        state.last_ping = Some(now);
    }
    ready
}

fn bind(app: &mut App) {
    if app.lan.socket.is_some() {
        return;
    }
    if !check_interval(&mut app.lan, Duration::from_millis(5000)) {
        return;
    }

    match make_socket() {
        Err(error) => {
            if error.kind() == ErrorKind::AddrInUse {
                set_status(app, "error: address already in use");
            } else {
                set_status(app, format!("error: {:?}", error));
            }
        }
        Ok(socket) => {
            app.lan.socket = Some(socket);
            update_local_ip(app);
            
            set_status(app, "connected");
        }
    }
}

fn make_socket() -> IOResult<UdpSocket> {
    let socket = UdpSocket::bind(("0.0.0.0", PORT))?;
    socket.set_broadcast(true)?;
    socket.set_read_timeout(Some(Duration::from_millis(50)))?;
    Ok(socket)
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

fn send_ping(socket: &UdpSocket, local_name: &str) -> IOResult<()> {
    let len = min(local_name.len(), u8::max_value() as usize);
    let mut message = vec![len as u8];
    message.extend_from_slice(&local_name.as_bytes()[0..len]);

    message.push(0);

    let port = 14u16;
    message.extend_from_slice(&port.to_be_bytes());

    socket.send_to(&message, ("255.255.255.255", PORT))?;
    Ok(())
}

/// Returns false when done.
fn receive_ping(app: &mut App) -> IOResult<()> {
    let socket = if let Some(ref socket) = app.lan.socket {
        socket
    } else {
        return Err(ErrorKind::NotConnected.into());
    };

    let mut buf = [0; 2048];
    let (count, source) = socket.recv_from(&mut buf)?;
    let ip = source.ip();
    if ip == IpAddr::from([127, 0, 0, 1]) {
        return Ok(());
    }

    let message = &buf[..count];

    let (name, _port) = if let Some(a) = parse_ping(message) {
        a
    } else {
        set_status(app, format!("invalid ping from {:?}", source));
        return Ok(());
    };
    set_status(app, format!("received from {:?}", source));

    if let Some(peer) = app.lan.peers.iter_mut().find(|a| a.address == ip) {
        peer.name.clear();
        peer.name.push_str(name);
    } else {
        app.lan.peers.push(Peer {
            name: name.to_string(),
            address: source.ip(),
        });
    }
    Ok(())
}

fn ping(app: &mut App) {
    if app.lan.socket.is_none() {
        return;
    }
    if !check_interval(&mut app.lan, Duration::from_millis(2000)) {
        return;
    }

    if let Some(ref socket) = app.lan.socket {
        if let Err(error) = send_ping(socket, &app.lan.local_name) {
            disconnect(app);
            set_status(app, format!("ping error: {:?}", error));
            return;
        }
    }

    for _ in 0..50 {
        if let Err(error) = receive_ping(app) {
            match error.kind() {
                ErrorKind::TimedOut | ErrorKind::WouldBlock => {}
                _ => {
                    set_status(app, format!("recv error: {:?}", error));
                    disconnect(app);
                }
            }
            break;
        }
    }
}

pub fn send(app: &mut App, message: String) {
    app.messages.push(Message {
        timestamp: now_fmt(),
        direction: MessageType::Sending,
        name: app.recipient.peer.name.clone(),
        content: message,
    });
}
