use std::net::UdpSocket;
use time::macros::format_description;
use time::OffsetDateTime;

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
    pub lan: LANState,
    pub recipient: RecipientState,
}

#[derive(Default)]
pub struct RecipientState {
    /// For remembering which peer to go back to if it's added back to the list.
    /// The length is 0 if no peer was selected.
    pub name: String,
    /// For remembering which peer to move onto if tabbing away from a missing peer.
    pub index: usize,
    /// False if the peer disappeared out of the list.
    pub valid: bool,
}

#[derive(Default)]
pub struct LANState {
    pub socket: Option<UdpSocket>,
    pub peers: Vec<Peer>,
    pub local_name: String,
}

pub struct Peer {
    pub name: String,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum MessageDirection {
    Sent,
    Received,
}

impl Default for MessageDirection {
    fn default() -> MessageDirection {
        MessageDirection::Sent
    }
}

#[derive(Default)]
pub struct Message {
    pub timestamp: String,
    pub direction: MessageDirection,
    pub name: String,
    pub content: String,
}

fn now_fmt() -> String {
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
        direction: MessageDirection::Sent,
        name,
        content,
    }
}

pub fn received(name: String, content: String) -> Message {
    Message {
        timestamp: now_fmt(),
        direction: MessageDirection::Received,
        name,
        content,
    }
}
