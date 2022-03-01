use std::{error::Error, io};
use std::mem::take;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Alignment},
    Frame, Terminal,
};
use clipboard::{ClipboardProvider, ClipboardContext};

mod data;
use data::{App, InputMode};

mod network;
use network::network_update;

mod render;
use render::{ui_scrolling_list, render_input, ui_instructions, ui_info, ui_messages};

fn main() -> Result<(), Box<dyn Error>> {
    // set up terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::default();
    network_update(&mut app.lan);

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
    loop {
        if let InputMode::Normal = app.input_mode {
            // Partial fix for cursor still showing in Cygwin.
            // Implementation of terminal.draw may need reordered to fully fix it.
            terminal.hide_cursor()?;
        }

        terminal.draw(|f| ui(f, &app))?;

        input(&mut app)?;

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

//     ctx.set_contents(the_string.to_owned()).unwrap();

fn input(app: &mut App) -> Result<(), Box<dyn Error>> {
    if let Event::Key(key) = event::read()? {
        match (key.code, key.modifiers) {
            (KeyCode::Char('v'), KeyModifiers::ALT) => {
                paste(app)?;
                return Ok(());
            },
            (KeyCode::Tab, KeyModifiers::SHIFT) => {
                // NOTE: Shift+Tab doesn't work on the Windows Command Prompt
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
                return Ok(());
            },
            (KeyCode::Tab, KeyModifiers::NONE) => {
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
                return Ok(());
            },
            _ => {}
        }

        match app.input_mode {
            InputMode::Normal => match key.code {
                KeyCode::Enter => {
                    if app.recipient.valid {
                        app.input_mode = InputMode::Editing;
                    }
                }
                KeyCode::Char('q') => {
                    app.quitting = true;
                }
                KeyCode::Esc => {
                    app.input.clear();
                }
                _ => {}
            },
            InputMode::Editing => match key.code {
                KeyCode::Enter => {
                    app.messages.push(take(&mut app.input));
                }
                KeyCode::Char(c) => {
                    app.input.push(c);
                }
                KeyCode::Backspace => {
                    app.input.pop();
                }
                KeyCode::Esc => {
                    app.input_mode = InputMode::Normal;
                }
                _ => {}
            },
        }
    }
    Ok(())
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &App) {
    ////////////// layout

    let horiz = Layout::default()
        .direction(Direction::Horizontal)
        .vertical_margin(1)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(18),
            Constraint::Min(8),
        ].as_ref())
        .split(f.size());

    let side = Layout::default()
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(5),
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
            Constraint::Length(3),
        ].as_ref())
        .split(horiz[2]);

    let cell_input = vert[1];
    let cell_messages = vert[0];

    /////////////// widgets

    f.render_widget(ui_info(app).alignment(Alignment::Right), cell_info);

    let options = app.lan.peers.iter().map(|peer| peer.name.clone()).collect::<Vec<_>>();
    f.render_widget(ui_scrolling_list(10, "network:", &app.recipient.name, &options)
        .alignment(Alignment::Right), cell_peers);

    f.render_widget(ui_instructions(app.input_mode, app.recipient.valid, app.input.len() > 0),
        cell_instructions);

    render_input(f, app, cell_input);

    f.render_widget(ui_messages(app), cell_messages);
}
