use std::net::UdpSocket;
use gethostname::gethostname;

#[derive(Default)]
pub struct LANState {
    socket: Option<UdpSocket>,
    pub peers: Vec<Peer>,
    pub local_name: String,
}

pub struct Peer {
    pub name: String,
}

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
}

// pub fn ping()

// use std::net::{UdpSocket, SocketAddr};

// const PORT: u16 = 62634;

// fn main() {
// let socket = UdpSocket::bind("0.0.0.0:8477")?;

// let send_result = socket.send_to(&message, "255.255.255.255:8477");
//             if let Err(error) = send_result {
//                 once! {
//                     handle(domain, &mut EventError::from_err(error, "sync"));
//                 }

// }

// //         if error.kind() == ErrorKind::AddrInUse {
// //             if event.attempts == 4 {
// //                 warn!("4 attempts and broadcast socket still in use");
// //             }
// //         } else {
// //             once! {
// //                 warn!("broadcast socket: {}", error);
// //             }
// //         }
// // fn try_init_broadcast(domain: &mut Domain) -> IoResult<()> {
// //     let socket = UdpSocket::bind("0.0.0.0:8477")?;
// //     socket.set_broadcast(true)?;

// //     // Timeout is workaround for these issues:
// //     // https://github.com/rust-lang/rfcs/issues/957
// //     // https://github.com/rust-lang/rust/issues/23272
// //     // ^ Can't use shutdown signal to interrupt reading thread.
// //     socket.set_read_timeout(Some(BROADCAST_INTERVAL))?;
// //     let socket_b = socket.try_clone()?;

// //     info!("Broadcast ready.");

// //     let mut proceed = ProceedSubject::new();
// //     let observer = proceed.make_observer();

// //     let trigger = make_entity(domain);
// //     let id_key = generate_id_key();
// //     set_data(domain, BroadcastData {
// //         id_key: id_key.clone(),
// //         trigger,
// //         socket: Some(socket_b),
// //         _proceed: proceed,
// //     });
// //     init_timer_imprecise(domain, trigger, BROADCAST_INTERVAL);
// //     init_uhandler(domain, handle_timer);

// //     let to_domain = remote_signaller_of(domain);
// //     let source = SourceBroadcast::new(socket, id_key);
// //     let source = source_proceeds(source, observer);
// //     let source = show_source_error(source);
// //     thread_read_signals(to_domain, source, "broadcast".into()).map(|_join_handle| ())
