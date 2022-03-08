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
use crate::data::{App, InputMode, sent, received};
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
