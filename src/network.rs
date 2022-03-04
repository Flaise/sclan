use std::net::UdpSocket;
use std::time::{Duration, Instant};
use std::io::{ErrorKind, Result as IOResult};
use gethostname::gethostname;
use crate::data::{LANState, Peer, set_status, App};

const PORT: u16 = 31331;

pub fn network_update(app: &mut App) {
    if app.lan.local_name.len() == 0 {
        app.lan.local_name = gethostname().into_string().unwrap_or("???".into());

        app.lan.peers.push(Peer {name: "yeah".into()});
        app.lan.peers.push(Peer {name: "a".into()});
        app.lan.peers.push(Peer {name: "b".into()});
        app.lan.peers.push(Peer {name: "c".into()});
        app.lan.peers.push(Peer {name: "rrr".into()});
        app.lan.peers.push(Peer {name: "eqweqw".into()});
        app.lan.peers.push(Peer {name: "LLEL".into()});
        app.lan.peers.push(Peer {name: "213456".into()});
    }

    bind(app);
    ping(app);
}

fn check_interval(state: &mut LANState, interval: Duration) -> bool {
    let now = Instant::now();
    state.last_ping.map(|last| now.duration_since(last) >= interval).unwrap_or(true)
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
            app.lan.socket = None;
            set_status(app, format!("ping error: {:?}", error));
            return;
        }

        let mut buf = [0; 2048];
        match socket.recv_from(&mut buf) {
            Ok((count, _source)) => {
                let _message = &buf[..count];
                set_status(app, "received!");
            }
            Err(error) => {
                match error.kind() {
                    ErrorKind::TimedOut | ErrorKind::WouldBlock => {
                        set_status(app, "recv timeout");
                    }
                    _ => {
                        set_status(app, format!("recv error: {:?}", error));
                        app.lan.socket = None;
                    }
                }
            }
        }
    }
}
