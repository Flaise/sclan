use std::{error::Error, io};
use std::mem::take;
use std::time::Duration;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, read, poll},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Alignment, Rect},
    Frame, Terminal,
};
use clipboard::{ClipboardProvider, ClipboardContext};

mod data;
use data::{App, InputMode, sent, received};

mod network;
use network::network_update;

mod render;
use render::{ui_scrolling_list, render_input, ui_instructions, ui_info, ui_messages, ui_status};

fn main() -> Result<(), Box<dyn Error>> {
    // set up terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
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
    app.needs_redraw = true;

    loop {
        network_update(&mut app.lan);

        if app.needs_redraw {
            app.needs_redraw = false;

            if let InputMode::Normal = app.input_mode {
                // Partial fix for cursor still showing in Cygwin.
                // Implementation of terminal.draw may need reordered to fully fix it.
                terminal.hide_cursor()?;
            }

            terminal.draw(|f| ui(f, &app))?;
        }

        input(&mut app, Duration::from_millis(500))?;

        if app.quitting {
            return Ok(());
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

fn input(app: &mut App, timeout: Duration) -> Result<(), Box<dyn Error>> {
    let key = if poll(timeout)? {
        if let Event::Key(key) = read()? {
            key
        } else {
            return Ok(());
        }
    } else {
        return Ok(());
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
                if app.recipient.name.len() == 0 {
                    app.recipient.index = app.lan.peers.len() - 1;
                } else {
                    if app.recipient.index == 0 {
                        app.recipient.index = app.lan.peers.len();
                    }
                    app.recipient.index -= 1;
                }
                app.recipient.name = app.lan.peers[app.recipient.index].name.clone();
                app.recipient.valid = true;
            }
        }
        (_, KeyCode::Tab, KeyModifiers::NONE) => {
            if app.lan.peers.len() > 0 {
                if app.recipient.name.len() == 0 {
                    app.recipient.index = 0;
                } else {
                    app.recipient.index += 1;
                    if app.recipient.index >= app.lan.peers.len() {
                        app.recipient.index = 0;
                    }
                }
                app.recipient.name = app.lan.peers[app.recipient.index].name.clone();
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
                app.messages.push(sent(app.recipient.name.clone(), content));
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

struct Cells {
    cell_info: Rect,
    cell_peers: Rect,
    cell_instructions: Rect,
    cell_input: Rect,
    cell_messages: Rect,
    cell_status: Rect,
}

fn calc_layout(base: Rect) -> Cells {
    let horiz = Layout::default()
        .direction(Direction::Horizontal)
        .vertical_margin(1)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(18),
            Constraint::Min(8),
        ].as_ref())
        .split(base);

    let side = Layout::default()
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(8),
        ])
        .split(horiz[1]);

    let cell_info = side[0];
    let cell_peers = side[1];
    let cell_instructions = side[2];

    let vert = Layout::default()
        .direction(Direction::Vertical)
        .horizontal_margin(1)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(1),
            Constraint::Length(3),
        ].as_ref())
        .split(horiz[2]);

    let cell_messages = vert[0];
    let cell_status = vert[1];
    let cell_input = vert[2];

    Cells {cell_info, cell_peers, cell_instructions, cell_input, cell_messages, cell_status}
}

fn ui<B: Backend>(frame: &mut Frame<B>, app: &App) {
    let Cells {cell_info, cell_peers, cell_instructions, cell_input, cell_messages, cell_status} =
        calc_layout(frame.size());

    frame.render_widget(ui_info(app).alignment(Alignment::Right), cell_info);

    let options = app.lan.peers.iter().map(|peer| peer.name.clone()).collect::<Vec<_>>();
    frame.render_widget(ui_scrolling_list(cell_peers, "network:", &app.recipient.name, &options)
        .alignment(Alignment::Right), cell_peers);

    frame.render_widget(ui_instructions(
        app.input_mode, app.recipient.valid, app.input.trim().len() > 0, app.messages.len() > 0,
        app.message_highlight.is_some()
    ), cell_instructions);

    frame.render_widget(ui_status(app), cell_status);

    render_input(frame, app, cell_input);

    frame.render_widget(ui_messages(app, cell_messages), cell_messages);
}
