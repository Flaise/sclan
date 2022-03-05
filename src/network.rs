use std::net::{UdpSocket, IpAddr};
use std::cmp::min;
use std::time::{Duration, Instant};
use std::io::{ErrorKind, Result as IOResult};
use std::str::from_utf8;
use gethostname::gethostname;
use crate::data::{LANState, Peer, set_status, App};

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

fn read_ping(message: &[u8]) -> Option<&str> {
    let len = if let Some(len) = message.get(0) {
        *len
    } else {
        return None;
    };
    let bytes = if let Some(bytes) = message.get(1..1 + len as usize) {
        bytes
    } else {
        return None;
    };
    if let Ok(name) = from_utf8(bytes) {
        Some(name)
    } else {
        None
    }
}

fn send_ping(socket: &UdpSocket, local_name: &str) -> IOResult<()> {
    let len = min(local_name.len(), u8::max_value() as usize);
    let mut message = vec![len as u8];
    message.extend_from_slice(&local_name.as_bytes()[0..len]);

    socket.send_to(&message, ("255.255.255.255", PORT))?;
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

    let mut buf = [0; 2048];
    loop {
        let socket = if let Some(ref socket) = app.lan.socket {
            socket
        } else {
            break;
        };

        match socket.recv_from(&mut buf) {
            Ok((count, source)) => {
                let ip = source.ip();
                if ip == IpAddr::from([127, 0, 0, 1]) {
                    continue;
                }

                let message = &buf[..count];

                let name = if let Some(name) = read_ping(message) {
                    name
                } else {
                    set_status(app, format!("invalid ping from {:?}", source));
                    continue;
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
            }
            Err(error) => {
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
}
