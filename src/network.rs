use std::net::{UdpSocket, IpAddr, TcpListener};
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

    bind_tcp(app);
    bind_udp(app);
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

fn bind_udp(app: &mut App) {
    if app.lan.udp_socket.is_some() {
        return;
    }
    if !check_interval(&mut app.lan, Duration::from_millis(3000)) {
        return;
    }

    match make_udp_socket() {
        Err(error) => {
            if error.kind() == ErrorKind::AddrInUse {
                set_status(app, "UDP error: address already in use");
            } else {
                set_status(app, format!("UDP error: {:?}", error));
            }
        }
        Ok(socket) => {
            app.lan.udp_socket = Some(socket);
            update_local_ip(app);
            
            set_status(app, "UDP ready");
        }
    }
}

fn disconnect_udp(app: &mut App) {
    app.lan.udp_socket = None;
    app.lan.local_addr.clear();
}

fn make_udp_socket() -> IOResult<UdpSocket> {
    let socket = UdpSocket::bind(("0.0.0.0", PORT))?;
    socket.set_broadcast(true)?;
    socket.set_read_timeout(Some(Duration::from_millis(50)))?;
    Ok(socket)
}

fn disconnect_tcp(app: &mut App) {
    app.lan.tcp_server = None;
    app.lan.local_addr.clear();
}

fn gimme_tcp_server_now() -> IOResult<TcpListener> {
    let server = TcpListener::bind("0.0.0.0:0")?;
    server.set_nonblocking(true)?;
    Ok(server)
}

fn bind_tcp(app: &mut App) {
    if app.lan.tcp_server.is_some() {
        return;
    }
    if !check_interval(&mut app.lan, Duration::from_millis(3000)) {
        return;
    }

    match gimme_tcp_server_now() {
        Err(error) => {
            set_status(app, format!("TCP listener error: {:?}", error));
        }
        Ok(server) => {
            app.lan.tcp_server = Some(server);
            update_local_ip(app);
            
            set_status(app, "TCP listener ready");
        }
    }
}

fn parse_ping(message: &[u8]) -> Option<(&str, u16)> {
    let len = if let Some(len) = message.get(0) {
        *len
    } else {
        return None;
    };

    let name_bytes = if let Some(a) = message.get(1..1 + len as usize) {
        a
    } else {
        return None;
    };

    let name = if let Ok(a) = from_utf8(name_bytes) {
        a
    } else {
        return None;
    };

    if message.get(1 + len as usize) != Some(&0) {
        // use zero termination just to make it easier to catch malformed packets
        return None;
    }

    let port_index = 2 + len as usize;
    let port_bytes = if let Some(a) = message.get(port_index..port_index + 2) {
        a
    } else {
        return None;
    };

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
fn receive_ping(app: &mut App) -> bool {
    let socket = if let Some(ref socket) = app.lan.udp_socket {
        socket
    } else {
        return false;
    };

    let mut buf = [0; 2048];
    match socket.recv_from(&mut buf) {
        Ok((count, source)) => {
            let ip = source.ip();
            if ip == IpAddr::from([127, 0, 0, 1]) {
                return true;
            }

            let message = &buf[..count];

            let (name, tcp_port) = if let Some(a) = parse_ping(message) {
                a
            } else {
                set_status(app, format!("invalid ping from {:?}", source));
                return true;
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
            true
        }
        Err(error) => {
            match error.kind() {
                ErrorKind::TimedOut | ErrorKind::WouldBlock => {}
                _ => {
                    disconnect_udp(app);
                    set_status(app, format!("recv error: {:?}", error));
                }
            }
            return false;
        }
    }
}

fn ping(app: &mut App) {
    if app.lan.udp_socket.is_none() {
        return;
    }
    if !check_interval(&mut app.lan, Duration::from_millis(2000)) {
        return;
    }

    if let Some(ref socket) = app.lan.udp_socket {
        if let Err(error) = send_ping(socket, &app.lan.local_name) {
            disconnect_udp(app);
            set_status(app, format!("ping error: {:?}", error));
            return;
        }
    }

    for _ in 0..50 {
        if !receive_ping(app) {
            return;
        }
    }
}
