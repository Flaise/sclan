use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{error::Error, io};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use unicode_width::UnicodeWidthStr;

mod network;
use network::{LANState, network_update};

mod render;
use render::ui_scrolling_list;

#[derive(Copy, Clone, PartialEq, Eq)]
enum InputMode {
    Normal,
    Editing,
}

impl Default for InputMode {
    fn default() -> InputMode {
        InputMode::Normal
    }
}

#[derive(Default)]
struct App {
    quitting: bool,
    input: String,
    input_mode: InputMode,
    messages: Vec<String>,
    lan: LANState,
    recipient: RecipientState,
}

#[derive(Default)]
struct RecipientState {
    /// For remembering which peer to go back to if it's added back to the list.
    /// The length is 0 if no peer was selected.
    name: String,
    /// For remembering which peer to move onto if tabbing away from a missing peer.
    index: usize,
    /// False if the peer disappeared out of the list.
    valid: bool,
}

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

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
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

fn input(app: &mut App) -> io::Result<()> {
    if let Event::Key(key) = event::read()? {
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
                KeyCode::Tab => {
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
    Ok(())
}

fn ui_instructions(input_mode: InputMode, recipient_valid: bool) -> Paragraph<'static> {
    let mut lines = vec![];
    if input_mode == InputMode::Normal {
        lines.push(Spans::from(vec![
            Span::styled("  [Tab]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("-recipient"),
        ]));
    } else {
        lines.push(Spans::default());
    }
    
    if !recipient_valid {
        lines.push(Spans::default());
    } else if input_mode == InputMode::Normal {
        lines.push(Spans::from(vec![
            Span::styled("[Enter]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("-write"),
        ]));
    } else {
        lines.push(Spans::from(vec![
            Span::styled("[Enter]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("-send"),
        ]));
    }

    if input_mode == InputMode::Normal {
        lines.push(Spans::from(vec![
            Span::styled("    [q]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("-quit"),
        ]));
    } else {
        lines.push(Spans::from(vec![
            Span::styled("  [Esc]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("-cancel"),
        ]));
    }

    Paragraph::new(lines)
}

fn render_input<B: Backend>(f: &mut Frame<B>, app: &App, cell_input: Rect) {
    let mut input_block = Block::default()
        .borders(Borders::ALL);
    if !app.recipient.valid {
        input_block = input_block.title(" Select a recipient. ");
    } else if app.input_mode == InputMode::Editing {
        let send_to = Spans::from(format!(" sending to: {} ", app.recipient.name));
        input_block = input_block.title(send_to);
    }
    let input = Paragraph::new(app.input.as_ref())
        .style(match app.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Editing => Style::default().fg(Color::Yellow),
        })
        .block(input_block);

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
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &App) {
    ////////////// layout

    let horiz = Layout::default()
        .direction(Direction::Horizontal)
        .vertical_margin(1)
        .constraints([
            Constraint::Min(8),
            Constraint::Length(18),
            Constraint::Length(1),
        ].as_ref())
        .split(f.size());

    let side = Layout::default()
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(4),
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
        .split(horiz[0]);

    let cell_input = vert[1];
    let cell_messages = vert[0];

    /////////////// widgets

    let info = Paragraph::new(vec![
        Spans::from("computer name:"),
        Spans::from(Span::styled(
            app.lan.local_name.clone(),
            Style::default().add_modifier(Modifier::BOLD)
        )),
    ]);
    f.render_widget(info, cell_info);

    let options = app.lan.peers.iter().map(|peer| peer.name.clone()).collect::<Vec<_>>();
    f.render_widget(ui_scrolling_list(8, "network:", &app.recipient.name, &options), cell_peers);

    f.render_widget(ui_instructions(app.input_mode, app.recipient.valid), cell_instructions);

    render_input(f, app, cell_input);

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
