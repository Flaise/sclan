use std::net::{UdpSocket, IpAddr};
use std::time::{Duration, Instant};
use std::io::{ErrorKind, Result as IOResult};
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

    // app.lan.local_addr = format!("{:?}", socket.local_addr().unwrap().ip());
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
    if app.lan.socket.is_none() {
        if !check_interval(&mut app.lan, Duration::from_millis(5000)) {
            return;
        }

        match UdpSocket::bind(("0.0.0.0", PORT)) {
            Err(error) => {
                if error.kind() == ErrorKind::AddrInUse {
                    set_status(app, "bind error: address already in use");
                } else {
                    set_status(app, format!("bind error: {:?}", error));
                }
            }
            Ok(socket) => {
                socket.set_broadcast(true).unwrap();
                socket.set_read_timeout(Some(Duration::from_millis(50))).unwrap();

                app.lan.socket = Some(socket);
                update_local_ip(app);
                
                set_status(app, "connected");
            }
        }
    }
}

fn send_ping(socket: &UdpSocket) -> IOResult<()> {
    socket.send_to(&[3], ("255.255.255.255", PORT))?;
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
        if let Err(error) = send_ping(socket) {
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
                let _message = &buf[..count];
                set_status(app, format!("received from {:?}", source));

                let ip = source.ip();
                if let Some(peer) = app.lan.peers.iter_mut().find(|a| a.address == ip) {
                    // update name
                } else {
                    app.lan.peers.push(Peer {
                        // name: "???".into(),
                        // name: format!("{:")
                        name: source.ip().to_string(),
                        address: source.ip(),
                    });
                }
            }
            Err(error) => {
                match error.kind() {
                    ErrorKind::TimedOut | ErrorKind::WouldBlock => {
                        set_status(app, "recv timeout");
                    }
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
