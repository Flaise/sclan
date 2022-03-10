mod data;
mod network;
mod network_broadcast;
mod network_p2p;
mod render;
mod layout;
mod actions;

use std::error::Error;
use std::io::stdout;
use std::time::Duration;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::{backend::{Backend, CrosstermBackend}, Terminal};
use crate::data::{App, InputMode, load_offset};
use crate::layout::ui;
use crate::actions::{input_async, input_terminal};

fn main() -> Result<(), Box<dyn Error>> {
    // set up terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::default();
    load_offset(&mut app);
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
