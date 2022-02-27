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
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use unicode_width::UnicodeWidthStr;

mod network;
use network::{LANState, Peer};

enum InputMode {
    Normal,
    Editing,
}

struct App {
    input: String,
    input_mode: InputMode,
    messages: Vec<String>,
    lan: LANState,
}

impl App {
    fn new() -> App {
        App {
            input: String::new(),
            input_mode: InputMode::Normal,
            messages: Vec::new(),
            lan: LANState::new(),
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

    let mut app = App::new();
    app.lan.peers.push(Peer {name: "yeah".into()});
    app.lan.peers.push(Peer {name: "a".into()});
    app.lan.peers.push(Peer {name: "b".into()});
    app.lan.peers.push(Peer {name: "c".into()});

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
        .constraints([
            Constraint::Min(3),
            Constraint::Length(3),
        ].as_ref())
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
    let cell_side = top_cells[1];

    let mut side_rows = vec![
        Spans::from("computer name:"),
        Spans::from(Span::styled(
            app.lan.local_name.clone(),
            Style::default().add_modifier(Modifier::BOLD)
        )),
        Spans::default(),
        // Spans::from("network:"),
        Spans::from(Span::styled(
            "network:",
            Style::default().add_modifier(Modifier::UNDERLINED)
        )),
    ];

    for peer in app.lan.peers.iter() {
        side_rows.push(Spans::from(peer.name.clone()));
    }
    side_rows.push(Spans::default());
    side_rows.push(Spans::from(vec![
        Span::styled("[q]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("=quit "),
    ]));

    let side = Paragraph::new(side_rows);
    f.render_widget(side, cell_side);

    let msg = match app.input_mode {
        InputMode::Normal => 
            vec![
                // Span::raw("Press "),
                // Span::styled("[q]", Style::default().add_modifier(Modifier::BOLD)),
                // Span::raw(" = exit . "),
                Span::raw(" "),
                Span::styled("[Enter]", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("=write "),
            ],
        InputMode::Editing => 
            vec![
                // Span::raw("Press "),
                Span::raw(" "),
                Span::styled("[Enter]", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("=send  "),
                Span::styled("[Esc]", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("=cancel "),
            ],
    };
    // let text = Text::from(Spans::from(msg));
    // text.patch_style(style);
    // let help_message = Paragraph::new(text);
    // f.render_widget(help_message, cell_instructions);

    // let paragraph = Paragraph::new(text.clone())
    //     .style(Style::default().bg(Color::White).fg(Color::Black))
    //     .block(create_block("Left, no wrap"))
    //     .alignment(Alignment::Left);
    // .wrap(Wrap { trim: false })

    let input = Paragraph::new(app.input.as_ref())
        .style(match app.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Editing => Style::default().fg(Color::Yellow),
            //.add_modifier(Modifier::RAPID_BLINK),
        })
        .block(Block::default().borders(Borders::ALL).title(Spans::from(msg)));
    f.render_widget(input, cell_input);
    match app.input_mode {
        InputMode::Normal => {}

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
        List::new(messages).block(Block::default().borders(Borders::ALL));
    f.render_widget(messages, cell_messages);
}
