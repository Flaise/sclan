use std::net::UdpSocket;

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
    pub messages: Vec<String>,
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
