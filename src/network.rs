use std::net::UdpSocket;
use std::time::{Duration, Instant};
use std::io::{ErrorKind, Result as IOResult};
use gethostname::gethostname;
use crate::data::{LANState, Peer};

const PORT: u16 = 31331;

pub fn network_update(state: &mut LANState) {
    if state.local_name.len() == 0 {
        state.local_name = gethostname().into_string().unwrap_or("???".into());

        state.peers.push(Peer {name: "yeah".into()});
        state.peers.push(Peer {name: "a".into()});
        state.peers.push(Peer {name: "b".into()});
        state.peers.push(Peer {name: "c".into()});
        state.peers.push(Peer {name: "rrr".into()});
        state.peers.push(Peer {name: "eqweqw".into()});
        state.peers.push(Peer {name: "LLEL".into()});
        state.peers.push(Peer {name: "213456".into()});
    }

    bind(state);
    ping(state);
}

fn check_interval(state: &mut LANState, interval: Duration) -> bool {
    let now = Instant::now();
    state.last_ping.map(|last| now.duration_since(last) >= interval).unwrap_or(true)
}

fn bind(state: &mut LANState) {
    if state.socket.is_none() {
        if !check_interval(state, Duration::from_millis(5000)) {
            return;
        }

        match UdpSocket::bind(("0.0.0.0", PORT)) {
            Err(error) => {
                if error.kind() == ErrorKind::AddrInUse {
                }
                todo!("display error");
            }
            Ok(socket) => {
                socket.set_broadcast(true).unwrap();
                socket.set_read_timeout(Some(Duration::from_millis(50))).unwrap();

                state.socket = Some(socket);
            }
        }
    }
}

fn send_ping(socket: &UdpSocket) -> IOResult<()> {
    socket.send_to(&[3], ("255.255.255.255", PORT))?;
    Ok(())
}

fn ping(state: &mut LANState) {
    if state.socket.is_none() {
        return;
    }
    if !check_interval(state, Duration::from_millis(2000)) {
        return;
    }

    if let Some(ref socket) = state.socket {
        if let Err(error) = send_ping(socket) {
            match error.kind() {
                ErrorKind::TimedOut | ErrorKind::WouldBlock => {}
                _ => {
                    todo!("display error");
                    state.socket = None;
                }
            }
        }
    }
}
