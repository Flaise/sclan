use std::net::IpAddr;
use std::sync::mpsc::{Sender, Receiver};
use time::macros::format_description;
use time::OffsetDateTime;
use crate::network::{ToNet, FromNet};

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Editing,
}

impl Default for InputMode {
    fn default() -> InputMode {
        InputMode::Normal
    }
}

#[derive(Default)]
pub struct App {
    pub quitting: bool,
    pub input: String,
    pub input_mode: InputMode,
    pub messages: Vec<Message>,
    pub message_highlight: Option<u16>,
    pub lan: LANState,
    pub lan_io: Option<LANIOState>,
    pub recipient: RecipientState,
    pub needs_redraw: bool,
    pub status: String,
    pub last_message_id: u32,
}

pub fn set_status(app: &mut App, message: impl AsRef<str>) {
    app.status.clear();
    app.status.push_str(" ");
    app.status.push_str(message.as_ref());
    app.status.push_str(" ");
    app.needs_redraw = true;
}

pub struct RecipientState {
    /// False if the peer disappeared out of the list or no peer was selected.
    pub valid: bool,
    /// For remembering which peer to move onto if tabbing away from a missing peer.
    pub index: usize,
    pub peer: Peer,
}

impl Default for RecipientState {
    fn default() -> Self {
        RecipientState {
            valid: false,
            index: 0,
            peer: Peer {
                name: Default::default(),
                address: [0, 0, 0, 0].into(),
            },
        }
    }
}

#[derive(Default)]
pub struct LANState {
    pub peers: Vec<Peer>,
    pub local_name: String,
    pub local_addr: String,
}

pub struct LANIOState {
    pub to_lan: Sender<ToNet>,
    pub from_lan: Receiver<FromNet>,
}

#[derive(Clone)]
pub struct Peer {
    pub name: String,
    pub address: IpAddr,

    // TODO: last_seen
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum MessageType {
    Sent,
    Sending,
    SendFailed,
    Received,
}

impl Default for MessageType {
    fn default() -> MessageType {
        MessageType::Sent
    }
}

#[derive(Default)]
pub struct Message {
    pub timestamp: String,
    pub direction: MessageType,
    pub name: String,
    pub content: String,
    pub message_id: u32,
}

pub fn now_fmt() -> String {
    let desc = format_description!(
        "[hour padding:space]:[minute] [month padding:space]/[day padding:space]/[year]"
    );

    match OffsetDateTime::now_local() {
        Ok(a) => a.format(&desc).unwrap_or("<format error>".to_string()),
        Err(_) => "<time zone error>".to_string(),
    }
}

pub fn sent(name: String, content: String) -> Message {
    Message {
        timestamp: now_fmt(),
        direction: MessageType::Sent,
        name,
        content,
        message_id: 0,
    }
}

pub fn received(name: String, content: String) -> Message {
    Message {
        timestamp: now_fmt(),
        direction: MessageType::Received,
        name,
        content,
        message_id: 0,
    }
}
