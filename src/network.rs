use std::net::IpAddr;
use std::time::Duration;
use std::sync::mpsc::{Sender, Receiver, channel, TryRecvError};
use std::thread::Builder as ThreadBuilder;
use gethostname::gethostname;
use tokio::runtime::Builder as RuntimeBuilder;
use tokio::time::sleep;
use tokio::spawn;
use crate::data::{App, LANIOState};
use crate::network_broadcast::task_ping;
use crate::network_p2p::task_p2p;

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
    ShowError(String),
    ShowMessage {
        source: IpAddr,
        content: String,
    },
    SendFailed(u32),
    SendArrived(u32),
    Peer {
        name: String,
        address: IpAddr,
    },
}

pub enum ToNet {
    Send {
        message_id: u32,
        address: IpAddr,
        content: String,
    }
}

pub fn message_to_net(app: &mut App, message: ToNet) -> Result<(), ()> {
    if let Some(ref state) = app.lan_io {
        if let Err(_) = state.to_lan.send(message) {
            app.lan_io = None;
            return Err(());
        }
        return Ok(());
    }
    Err(())
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
        // This happens if the thread couldn't start.
        None
    }
}

/// false = disconnected
#[must_use]
pub fn show_status(to_app: &mut Sender<FromNet>, content: impl Into<String>) -> bool {
    if let Err(_) = to_app.send(FromNet::ShowStatus(content.into())) {
        return false;
    }
    true
}

/// false = disconnected
#[must_use]
pub fn show_error(to_app: &mut Sender<FromNet>, content: impl Into<String>) -> bool {
    if let Err(_) = to_app.send(FromNet::ShowError(content.into())) {
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
        let _ignore = show_error(&mut to_app_2, format!("error starting thread: {:?}", error));
        return;
    }

    app.lan_io = Some(LANIOState {to_lan, from_lan});
}

fn run_network(from_app: Receiver<ToNet>, mut to_app: Sender<FromNet>) {
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
                let c = spawn(task_p2p(from_app, to_app.clone()));

                for r in [a, b, c] {
                    r.await.expect("task panicked");
                }
            });
        }
        Err(error) => {
            let _ignore = show_error(&mut to_app, format!("error building runtime: {:?}", error));
        }
    }
}

async fn task_local_name(to_app: Sender<FromNet>) {
    // TODO: can the host name be changed at runtime or is this loop a waste of time?
    loop {
        let name = gethostname().into_string().unwrap_or("".into());
        if let Err(_) = to_app.send(FromNet::ShowLocalName(name)) {
            return;
        }
        sleep(Duration::from_secs(5)).await;
    }
}
