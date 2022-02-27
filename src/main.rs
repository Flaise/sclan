use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{error::Error, io};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use unicode_width::UnicodeWidthStr;

enum InputMode {
    Normal,
    Editing,
}

/// App holds the state of the application
struct App {
    /// Current value of the input box
    input: String,
    /// Current input mode
    input_mode: InputMode,
    /// History of recorded messages
    messages: Vec<String>,
}

impl Default for App {
    fn default() -> App {
        App {
            input: String::new(),
            input_mode: InputMode::Normal,
            messages: Vec::new(),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // set up terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let app = App::default();
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
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        if let InputMode::Normal = app.input_mode {
            // Partial fix for cursor still showing in Cygwin.
            // Implementation of terminal.draw may need reordered to fully fix it.
            terminal.hide_cursor()?;
        }

        terminal.draw(|f| ui(f, &app))?;

        if let Event::Key(key) = event::read()? {
            match app.input_mode {
                InputMode::Normal => match key.code {
                    KeyCode::Enter => {
                        app.input_mode = InputMode::Editing;
                    }
                    KeyCode::Char('q') => {
                        return Ok(());
                    }
                    _ => {}
                },
                InputMode::Editing => match key.code {
                    KeyCode::Enter => {
                        app.messages.push(app.input.drain(..).collect());
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
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Min(3),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(f.size());

    let cell_input = chunks[1];

    let top_cells = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(8),
            Constraint::Length(16),
        ].as_ref())
        .split(chunks[0]);

    let cell_messages = top_cells[0];
    let cell_instructions = top_cells[1];

    // let messages: Vec<ListItem> = app
    //     .messages
    //     .iter()
    //     .enumerate()
    //     .map(|(i, m)| {
    //         let content = vec![Spans::from(Span::raw(format!("{}: {}", i, m)))];
    //         ListItem::new(content)
    //     })
    //     .collect();
    // let messages =
    //     List::new(messages).block(Block::default().borders(Borders::ALL).title(""));
    // f.render_widget(messages, cell_messages);
    let msg = match app.input_mode {
        InputMode::Normal => 
            vec![
                // Span::raw("Press "),
                Span::styled("[q]", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" = exit . "),
                Span::styled("[Enter]", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" = write"),
            ],
        InputMode::Editing => 
            vec![
                // Span::raw("Press "),
                Span::styled("[Esc]", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" = stop editing . "),
                Span::styled("[Enter]", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(" = send"),
            ],
    };
    let text = Text::from(Spans::from(msg));
    // text.patch_style(style);
    let help_message = Paragraph::new(text);
    f.render_widget(help_message, cell_instructions);

    // let paragraph = Paragraph::new(text.clone())
    //     .style(Style::default().bg(Color::White).fg(Color::Black))
    //     .block(create_block("Left, no wrap"))
    //     .alignment(Alignment::Left);
    // .wrap(Wrap { trim: false })

    let input = Paragraph::new(app.input.as_ref())
        .style(match app.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Editing => Style::default().fg(Color::Yellow).add_modifier(Modifier::RAPID_BLINK),
        })
        .block(Block::default().borders(Borders::ALL).title(""));
    f.render_widget(input, cell_input);
    match app.input_mode {
        InputMode::Normal =>
            // Hide the cursor. `Frame` does this by default, so we don't need to do anything here
            {}

        InputMode::Editing => {
            // Make the cursor visible and ask tui-rs to put it at the specified coordinates after rendering
            f.set_cursor(
                // Put cursor past the end of the input text
                cell_input.x + app.input.width() as u16 + 1,
                // Move one line down, from the border to the input line
                cell_input.y + 1,
            )
        }
    }

    let messages: Vec<ListItem> = app
        .messages
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let content = vec![Spans::from(Span::raw(format!("{}: {}", i, m)))];
            ListItem::new(content)
        })
        .collect();
    let messages =
        List::new(messages).block(Block::default().borders(Borders::ALL).title(""));
    f.render_widget(messages, cell_messages);
}



// use std::{io, thread, time::Duration};
// use tui::{
//     backend::CrosstermBackend,
//     widgets::{Widget, Block, Borders},
//     layout::{Layout, Constraint, Direction},
//     Terminal
// };
// use crossterm::{
//     event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
//     execute,
//     terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
// };

// fn main() -> Result<(), io::Error> {
//     // setup terminal
//     enable_raw_mode()?;
//     let mut stdout = io::stdout();
//     execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
//     let backend = CrosstermBackend::new(stdout);
//     let mut terminal = Terminal::new(backend)?;

//     terminal.draw(|f| {
//         let size = f.size();
//         let block = Block::default()
//             .title("Block")
//             .borders(Borders::ALL);
//         f.render_widget(block, size);
//     })?;

//     thread::sleep(Duration::from_millis(5000));

//     // restore terminal
//     disable_raw_mode()?;
//     execute!(
//         terminal.backend_mut(),
//         LeaveAlternateScreen,
//         DisableMouseCapture
//     )?;
//     terminal.show_cursor()?;

//     Ok(())
// }


// use std::io::{stdout, Write};

// use crossterm::{
//     execute,
//     style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
//     ExecutableCommand, Result,
//     event,
// };

// fn main() -> Result<()> {
//     // using the macro
//     execute!(
//         stdout(),
//         SetForegroundColor(Color::Blue),
//         SetBackgroundColor(Color::Red),
//         Print("Styled text here."),
//         ResetColor
//     )?;

//     // or using functions
//     stdout()
//         .execute(SetForegroundColor(Color::Blue))?
//         .execute(SetBackgroundColor(Color::Red))?
//         .execute(Print("Styled text here."))?
//         .execute(ResetColor)?;
    
//     Ok(())
// }

// use std::net::{UdpSocket, SocketAddr};

// const PORT: u16 = 62634;

// fn main() {
// let socket = UdpSocket::bind("0.0.0.0:8477")?;

// let send_result = socket.send_to(&message, "255.255.255.255:8477");
//             if let Err(error) = send_result {
//                 once! {
//                     handle(domain, &mut EventError::from_err(error, "sync"));
//                 }

// }

// //         if error.kind() == ErrorKind::AddrInUse {
// //             if event.attempts == 4 {
// //                 warn!("4 attempts and broadcast socket still in use");
// //             }
// //         } else {
// //             once! {
// //                 warn!("broadcast socket: {}", error);
// //             }
// //         }
// // fn try_init_broadcast(domain: &mut Domain) -> IoResult<()> {
// //     let socket = UdpSocket::bind("0.0.0.0:8477")?;
// //     socket.set_broadcast(true)?;

// //     // Timeout is workaround for these issues:
// //     // https://github.com/rust-lang/rfcs/issues/957
// //     // https://github.com/rust-lang/rust/issues/23272
// //     // ^ Can't use shutdown signal to interrupt reading thread.
// //     socket.set_read_timeout(Some(BROADCAST_INTERVAL))?;
// //     let socket_b = socket.try_clone()?;

// //     info!("Broadcast ready.");

// //     let mut proceed = ProceedSubject::new();
// //     let observer = proceed.make_observer();

// //     let trigger = make_entity(domain);
// //     let id_key = generate_id_key();
// //     set_data(domain, BroadcastData {
// //         id_key: id_key.clone(),
// //         trigger,
// //         socket: Some(socket_b),
// //         _proceed: proceed,
// //     });
// //     init_timer_imprecise(domain, trigger, BROADCAST_INTERVAL);
// //     init_uhandler(domain, handle_timer);

// //     let to_domain = remote_signaller_of(domain);
// //     let source = SourceBroadcast::new(socket, id_key);
// //     let source = source_proceeds(source, observer);
// //     let source = show_source_error(source);
// //     thread_read_signals(to_domain, source, "broadcast".into()).map(|_join_handle| ())
