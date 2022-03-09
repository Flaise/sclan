use std::error::Error;
use std::mem::take;
use std::time::Duration;
use std::net::IpAddr;
use crossterm::event::{Event, KeyCode, KeyModifiers, read, poll};
use clipboard::{ClipboardProvider, ClipboardContext};
use crate::data::{App, InputMode, now_fmt, Message, MessageType, set_status, Peer};
use crate::network::{ToNet, message_to_net, message_from_net, FromNet};

pub fn input_async(app: &mut App) {
    // TODO: cull idle peers

    while let Some(message) = message_from_net(app) {
        match message {
            FromNet::ShowStatus(content) => set_status(app, false, content),
            FromNet::ShowError(content) => {
                set_status(app, true, &content);
                show_error(app, content);
            }
            FromNet::ShowLocalName(name) => app.lan.local_name = name,
            FromNet::ShowLocalAddress(addr) => app.lan.local_addr = addr,
            FromNet::Peer {name, address} => {
                if let Some(peer) = app.lan.peers.iter_mut().find(|a| a.address == address) {
                    peer.name.clear();
                    peer.name.push_str(&name);
                } else {
                    app.lan.peers.push(Peer {
                        name: name.to_string(),
                        address,
                    });
                }
            }
            FromNet::SendFailed(message_id) => {
                update_message(app, message_id, MessageType::SendFailed);
            }
            FromNet::SendArrived(message_id) => {
                update_message(app, message_id, MessageType::Sent);
            }
            FromNet::ShowMessage {source, content} => show_message(app, source, content),
        }
        app.needs_redraw = true;
    }
}

pub fn input_terminal(app: &mut App, timeout: Duration) -> Result<(), Box<dyn Error>> {
    if !poll(timeout)? {
        return Ok(());
    }
    let key = match read()? {
        Event::Key(key) => key,
        Event::Resize(_, _) => {
            app.needs_redraw = true;
            return Ok(());
        }
        _ => return Ok(()),
    };

    match (app.input_mode, key.code, key.modifiers) {
        (InputMode::Normal, KeyCode::Char('c'), KeyModifiers::ALT) => {
            copy(app)?;
        }
        (_, KeyCode::Char('v'), KeyModifiers::ALT) => {
            paste(app)?;
        }
        (_, KeyCode::Tab, KeyModifiers::SHIFT) => {
            // NOTE: Shift+Tab doesn't work on the Windows Command Prompt
            // https://stackoverflow.com/questions/6129143/how-to-map-shift-tab-in-vim-cygwin-windows-cmd-exe#6129580
            if app.lan.peers.len() > 0 {
                if !app.recipient.valid {
                    app.recipient.index = app.lan.peers.len() - 1;
                } else {
                    if app.recipient.index == 0 {
                        app.recipient.index = app.lan.peers.len();
                    }
                    app.recipient.index -= 1;
                }
                app.recipient.peer = app.lan.peers[app.recipient.index].clone();
                app.recipient.valid = true;
            }
        }
        (_, KeyCode::Tab, KeyModifiers::NONE) => {
            if app.lan.peers.len() > 0 {
                if !app.recipient.valid {
                    app.recipient.index = 0;
                } else {
                    app.recipient.index += 1;
                    if app.recipient.index >= app.lan.peers.len() {
                        app.recipient.index = 0;
                    }
                }
                app.recipient.peer = app.lan.peers[app.recipient.index].clone();
                app.recipient.valid = true;
            }
        }
        (InputMode::Normal, KeyCode::Enter, _) => {
            if app.recipient.valid {
                app.input_mode = InputMode::Editing;
                app.message_highlight = None; // TODO: this should be an InputMode
            }
        }
        (InputMode::Normal, KeyCode::Char('q'), _) => {
            app.quitting = true;
        }
        (InputMode::Normal, KeyCode::Esc, _) => {
            if app.message_highlight.is_some() {
                app.message_highlight = None;
            } else {
                app.input.clear();
            }
        }

        (InputMode::Normal, KeyCode::Up, _) => {
            if app.messages.len() > 0 {
                match app.message_highlight {
                    None => app.message_highlight = Some(app.messages.len() as u16 - 1),
                    Some(0) => {}
                    Some(old) => app.message_highlight = Some(old - 1),
                }
            }
        }
        (InputMode::Normal, KeyCode::Down, _) => {
            if app.messages.len() > 0 {
                match app.message_highlight {
                    None => app.message_highlight = Some(app.messages.len() as u16 - 1),
                    Some(old) => {
                        if old < app.messages.len() as u16 - 1 {
                            app.message_highlight = Some(old + 1);
                        }
                    }
                }
            }
        }

        (InputMode::Editing, KeyCode::Enter, _) => {
            if app.input.trim().len() > 0 {
                let content = take(&mut app.input);
                send(app, content);
            } else {
                app.input.clear();
                app.input_mode = InputMode::Normal;
            }
        }
        (InputMode::Editing, KeyCode::Char(c), k) => {
            if !k.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) {
                app.input.push(c);
            }
        }
        (InputMode::Editing, KeyCode::Backspace, _) => {
            app.input.pop();
        }
        (InputMode::Editing, KeyCode::Esc, _) => {
            app.input_mode = InputMode::Normal;
        }
        _ => {
            return Ok(());
        }
    }
    
    app.needs_redraw = true;
    Ok(())
}

fn update_message(app: &mut App, message_id: u32, new_type: MessageType) {
    for message in &mut app.messages {
        if message.message_id == message_id {
            message.direction = new_type;
            app.needs_redraw = true;
            return;
        }
    }
}

fn show_error(app: &mut App, content: String) {
    app.messages.push(Message {
        timestamp: now_fmt(),
        direction: MessageType::Error,
        name: "".into(),
        content,
        message_id: 0,
    });
}

fn show_message(app: &mut App, address: IpAddr, content: String) {
    let name = if let Some(peer) = app.lan.peers.iter_mut().find(|a| a.address == address) {
        peer.name.clone()
    } else {
        address.to_string()
    };

    app.messages.push(Message {
        timestamp: now_fmt(),
        direction: MessageType::Received,
        name,
        // TODO: update old messages when a peer becomes named
        // TODO: maybe also save source address so they can RE-name with the peer
        content,
        message_id: 0,
    });
}

fn next_message_id(app: &mut App) -> u32 {
    app.last_message_id = app.last_message_id.wrapping_add(1);
    app.last_message_id
}

fn send(app: &mut App, content: String) {
    if app.recipient.valid {
        let message_id = next_message_id(app);

        app.messages.push(Message {
            timestamp: now_fmt(),
            direction: MessageType::Sending,
            name: app.recipient.peer.name.clone(),
            content: content.clone(),
            message_id,
        });

        if let Err(_) = message_to_net(app, ToNet::Send {
            message_id,
            address: app.recipient.peer.address,
            content,
        }) {
            update_message(app, message_id, MessageType::SendFailed);
        }
    }
}

fn paste(app: &mut App) -> Result<(), Box<dyn Error>> {
    let mut ctx: ClipboardContext = ClipboardProvider::new()?;
    let mut stuff = ctx.get_contents()?;
    app.input.extend(stuff.drain(..));
    Ok(())
}

fn copy(app: &mut App) -> Result<(), Box<dyn Error>> {
    if let Some(index) = app.message_highlight {
        let content = app.messages[index as usize].content.clone();
        let mut ctx: ClipboardContext = ClipboardProvider::new()?;
        ctx.set_contents(content)?;
    }
    Ok(())
}
