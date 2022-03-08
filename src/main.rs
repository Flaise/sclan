mod data;
mod network;
mod render;
mod layout;

use std::error::Error;
use std::io::stdout;
use std::mem::take;
use std::time::Duration;
use std::net::IpAddr;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, read, poll},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::{backend::{Backend, CrosstermBackend}, Terminal};
use clipboard::{ClipboardProvider, ClipboardContext};
use crate::data::{App, InputMode, sent, received, now_fmt, Message, MessageType, set_status, Peer};
use crate::network::{ToNet, message_to_net, message_from_net, FromNet};
use crate::layout::ui;

fn main() -> Result<(), Box<dyn Error>> {
    // set up terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::default();

    app.messages.push(sent("argv".into(),
        "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.".into()));
    app.messages.push(received("argv".into(),
        "Platea dictumst quisque sagittis purus.".into()));
    app.messages.push(sent("yeah".into(),
        "Varius vel pharetra vel turpis nunc eget lorem dolor.".into()));
    app.messages.push(sent("well ok then?".into(),
        "Nisi est sit amet facilisis magna etiam tempor orci. Id eu nisl nunc mi ipsum faucibus vitae aliquet.".into()));
    app.messages.push(received("yeah".into(),
        "Ut tristique et egestas quis ipsum.".into()));
    app.messages.push(received("yeah".into(),
        "Interdum velit laoreet id donec.".into()));
    app.messages.push(sent("argv".into(),
        "Convallis convallis tellus id interdum velit laoreet.".into()));
    app.messages.push(received("another computer".into(),
        "* Tellus mauris a diam maecenas sed.\n* Ultricies tristique nulla aliquet enim tortor at auctor urna.\n* Malesuada nunc vel risus commodo viverra maecenas.".into()));
    app.messages.push(received("none".into(),
        "Libero volutpat sed cras ornare arcu dui vivamus arcu felis.".into()));
    app.messages.push(sent("another computer".into(),
        "Ut aliquam purus sit amet luctus venenatis. Vitae justo eget magna fermentum iaculis eu non. Velit aliquet sagittis id consectetur purus ut.".into()));

    let res = run_app(&mut terminal, app);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> Result<(), Box<dyn Error>> {
    let app = &mut app;
    app.needs_redraw = true;

    loop {
        input_async(app);

        if app.needs_redraw {
            app.needs_redraw = false;

            if let InputMode::Normal = app.input_mode {
                // Partial fix for cursor still showing in Cygwin.
                // Implementation of terminal.draw may need reordered to fully fix it.
                terminal.hide_cursor()?;
            }

            terminal.draw(|f| ui(f, app))?;
        }

        input_terminal(app, Duration::from_millis(500))?;

        if app.quitting {
            return Ok(());
        }
    }
}

fn input_async(app: &mut App) {
    while let Some(message) = message_from_net(app) {
        match message {
            FromNet::ShowStatus(content) => set_status(app, content),
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
            FromNet::SendFailed(message_id) => send_failed(app, message_id),
            FromNet::SendArrived(message_id) => send_arrived(app, message_id),
            FromNet::ShowMessage {source, content} => show_message(app, source, content),
        }
        app.needs_redraw = true;
    }
}

fn input_terminal(app: &mut App, timeout: Duration) -> Result<(), Box<dyn Error>> {
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

fn send_arrived(app: &mut App, message_id: u32) {
    for message in &mut app.messages {
        if message.message_id == message_id {
            message.direction = MessageType::Sent;
            app.needs_redraw = true;
            return;
        }
    }
}

fn send_failed(app: &mut App, message_id: u32) {
    for message in &mut app.messages {
        if message.message_id == message_id {
            message.direction = MessageType::SendFailed;
            app.needs_redraw = true;
            return;
        }
    }
}

fn show_message(app: &mut App, source: IpAddr, content: String) {

}

fn send(app: &mut App, content: String) {
    if app.recipient.valid {
        app.last_message_id = app.last_message_id.wrapping_add(1);
        let message_id = app.last_message_id;

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
            send_failed(app, message_id);
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
