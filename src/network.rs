use std::net::IpAddr;
use std::cmp::min;
use std::time::Duration;
use std::io::{ErrorKind, Result as IOResult, Error as IOError};
use std::str::from_utf8;
use std::sync::mpsc::{Sender, Receiver, channel, TryRecvError};
use std::thread::Builder as ThreadBuilder;
use std::sync::Arc;
use gethostname::gethostname;
use tokio::runtime::Builder as RuntimeBuilder;
use tokio::time::sleep;
use tokio::{spawn, select};
use tokio::net::UdpSocket;
use crate::data::{App, LANIOState};

const PORT: u16 = 31331;

// struct LANInternal {
//     socket: Option<UdpSocket>,
//     to_app: Sender<FromNet>,
//     from_app: Receiver<ToNet>,
// }

                // let state = Arc::new(LANInternal {
                //     socket: None,
                //     to_app,
                //     from_app,
                // });

pub enum FromNet {
    ShowLocalName(String),
    ShowLocalAddress(String),
    ShowStatus(String),
    MessageFailed(u32),
    MessageArrived(u32),
    Peer {
        name: String,
        address: IpAddr,
    }
}

pub enum ToNet {
    Send {
        message_id: u32,
        address: IpAddr,
        content: String,
    }
}

pub fn message_to_net(_app: &mut App, _message: ToNet) {
    // TODO
}

pub fn message_from_net(app: &mut App) -> Option<FromNet> {
    if app.lan_io.is_none() {
        start_network(app);
    }
    if let Some(ref state) = app.lan_io {
        match state.from_lan.try_recv() {
            Ok(message) => Some(message),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                app.lan_io = None;
                None
            }
        }
    } else {
        debug_assert!(false, "should be unreachable");
        None
    }
}

/// false = disconnected
#[must_use]
fn show_status(to_app: &mut Sender<FromNet>, content: impl Into<String>) -> bool {
    if let Err(_) = to_app.send(FromNet::ShowStatus(content.into())) {
        return false;
    }
    true
}

fn start_network(app: &mut App) {
    let (to_lan, from_app) = channel();
    let (mut to_app, from_lan) = channel();

    let _ignore = show_status(&mut to_app, "starting thread");
    let mut to_app_2 = to_app.clone();
    if let Err(error) = ThreadBuilder::new()
            .name("async".into())
            .spawn(move || run_network(from_app, to_app)) {
        let _ignore = show_status(&mut to_app_2, format!("error starting thread: {:?}", error));
    }

    app.lan_io = Some(LANIOState {
        to_lan,
        from_lan,
    });
}

fn run_network(_from_app: Receiver<ToNet>, mut to_app: Sender<FromNet>) {
    if !show_status(&mut to_app, "starting runtime") {
        return;
    }
    let runtime = RuntimeBuilder::new_current_thread()
        .enable_all()
        .build();
    match runtime {
        Ok(runtime) => {
            runtime.block_on(async {
                if !show_status(&mut to_app, "runtime started") {
                    return;
                }

                let a = spawn(task_local_name(to_app.clone()));
                let b = spawn(task_ping(to_app.clone()));
                a.await.unwrap();
                b.await.unwrap(); // TODO
            });
        }
        Err(error) => {
            let _ignore = show_status(&mut to_app, format!("error building runtime: {:?}", error));
        }
    }
}

async fn task_local_name(to_app: Sender<FromNet>) {
    loop {
        let name = gethostname().into_string().unwrap_or("???".into());
        if let Err(_) = to_app.send(FromNet::ShowLocalName(name)) {
            return;
        }
        sleep(Duration::from_secs(5)).await;
    }
}

async fn task_ping(mut to_app: Sender<FromNet>) {
    loop {
        let socket = match make_socket().await {
            Err(error) => {
                if error.kind() == ErrorKind::AddrInUse {
                    if !show_status(&mut to_app, "error: address already in use") {
                        return;
                    }
                } else {
                    if !show_status(&mut to_app, format!("error: {:?}", error)) {
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

        let pout = task_ping_out(socket.clone(), to_app.clone());
        let pin = task_ping_in(socket, to_app.clone());

        let done = select! {
            done = pout => done,
            done = pin => done,
        };
        match done {
            PingDone::Exiting => return,
            PingDone::IO(error) => {
                if !show_status(&mut to_app, format!("error: {:?}", error)) {
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

async fn task_ping_in(socket: Arc<UdpSocket>, mut to_app: Sender<FromNet>) -> PingDone {
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

        let (name, _port) = if let Some(a) = parse_ping(message) {
            a
        } else {
            if !show_status(to_app, format!("invalid ping from {:?}", source)) {
                return PingDone::Exiting;
            }
            continue;
        };
        if !show_status(to_app, format!("received from {:?}", source)) {
            return PingDone::Exiting;
        }

        let peer = FromNet::Peer {
            name: name.to_string(),
            address: source.ip(),
        };
        if let Err(_) = to_app.send(peer) {
            return PingDone::Exiting;
        }
    }
}

async fn task_ping_out(socket: Arc<UdpSocket>, mut to_app: Sender<FromNet>) -> PingDone {
    loop {
        let name = gethostname().into_string().unwrap_or("???".into());

        if let Err(error) = send_ping(&socket, &name).await {
            if !show_status(&mut to_app, format!("ping error: {:?}", error)) {
                return PingDone::Exiting;
            }
            return PingDone::IO(error);
        }

        sleep(Duration::from_secs(2)).await;
    }
}

fn local_ip() -> Option<IpAddr> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    Some(socket.local_addr().ok()?.ip())
}

fn show_local_ip(to_app: &mut Sender<FromNet>) {
    let addr = local_ip()
        .map(|a| a.to_string())
        .unwrap_or("???".to_string());
    let _ignore = to_app.send(FromNet::ShowLocalAddress(addr));
}

async fn make_socket() -> IOResult<UdpSocket> {
    let socket = UdpSocket::bind(("0.0.0.0", PORT)).await?;
    socket.set_broadcast(true)?;
    Ok(socket)
}

async fn send_ping(socket: &Arc<UdpSocket>, local_name: &str) -> IOResult<()> {
    let len = min(local_name.len(), u8::max_value() as usize);
    let mut message = vec![len as u8];
    message.extend_from_slice(&local_name.as_bytes()[0..len]);

    message.push(0);

    let port = 14u16;
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
