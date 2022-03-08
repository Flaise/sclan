use std::borrow::Cow;
use std::cmp::{min, max};
use std::iter::Iterator;
use tui::{backend::Backend, Frame};
use tui::widgets::{Paragraph, Block, Borders};
use tui::text::{Spans, Span};
use tui::style::{Style, Modifier, Color};
use tui::layout::{Alignment, Rect};
use unicode_width::UnicodeWidthStr;
use textwrap::wrap;
use crate::App;
use crate::data::{InputMode, MessageType, Message};

fn plain<'a, T>(message: T) -> Span<'a>
where T: Into<Cow<'a, str>> {
    Span::raw(message)
}

fn bold<'a, T>(message: T) -> Span<'a>
where T: Into<Cow<'a, str>> {
    Span::styled(message, Style::default().add_modifier(Modifier::BOLD))
}

fn reversed<'a, T>(message: T) -> Span<'a>
where T: Into<Cow<'a, str>> {
    Span::styled(message, Style::default().add_modifier(Modifier::REVERSED))
}

fn faded<'a, T>(message: T) -> Span<'a>
where T: Into<Cow<'a, str>> {
    Span::styled(message, Style::default().fg(Color::DarkGray))
}

// fn ui_scrollbar(width: u16, position: u16, count: u16) -> Spans<'static> {
//     let mut bar = vec![];
//     for i in 0..width {
//         let span = if i == position * count / width {
//             Span::raw("▓")
//         } else {
//             Span::raw("░")
//         };
//         bar.push(span);
//     }
//     Spans::from(bar)
// }

pub fn ui_scrolling_list(area: Rect, title: &str, selection: &str, options: &[String])
-> Paragraph<'static> {
    let max_options = max(1, area.height.saturating_sub(2) as usize);

    let mut lines = vec![
        Spans::from(title.to_string()),
    ];

    if let Some(index) = options.iter().position(|a| a == selection) {
        let label = format!("→ {}", options[index]);
        lines.push(Spans::from(reversed(label.clone())));

        for label in options.iter().skip(index + 1).take(max_options - 1) {
            lines.push(Spans::from(label.clone()));
        }
        if let Some(count) = min(options.len(), max_options).checked_sub(lines.len() - 1) {
            for label in options.iter().take(count) {
                lines.push(Spans::from(label.clone()));
            }
        }
    } else {
        for label in options.iter().take(max_options) {
            lines.push(Spans::from(label.clone()));
        }
    }

    if options.len() == 0 {
        lines.push(Spans::from(faded(" (searching...) ")));
    }

    while lines.len() < max_options {
        lines.push(Spans::default());
    }

    match options.len().checked_sub(lines.len() - 1) {
        Some(count) if count > 0 => lines.push(Spans::from(format!(" ({} more) …", count))),
        _ => lines.push(Spans::default()),
    }

    Paragraph::new(lines)
}

pub fn ui_instructions(input_mode: InputMode, recipient_valid: bool,
                       text_entered: bool, output_displayed: bool,
                       output_selected: bool) -> Paragraph<'static> {
    let mut lines = vec![];

    lines.push(Spans::from("__________________"));

    if input_mode == InputMode::Normal && output_displayed {
        lines.push(Spans::from(vec![bold(" [↑] [↓]"), plain("-message")]));
    } else {
        lines.push(Spans::default());
    }
    if output_selected {
        lines.push(Spans::from(vec![bold(" [Alt+C]"), plain("-copy")]));
    } else {
        lines.push(Spans::default());
    }

    lines.push(Spans::from(vec![bold(" [Alt+V]"), plain("-paste")]));

    lines.push(Spans::from(vec![bold("   [Tab]"), plain("-recipient")]));
    
    if !recipient_valid {
        lines.push(Spans::default());
    } else if input_mode == InputMode::Normal {
        lines.push(Spans::from(vec![bold(" [Enter]"), plain("-write")]));
    } else if text_entered {
        lines.push(Spans::from(vec![bold(" [Enter]"), plain("-send")]));
    } else {
        lines.push(Spans::default());
    }

    if input_mode == InputMode::Normal {
        if output_selected {
            lines.push(Spans::from(vec![bold("   [Esc]"), plain("-deselect")]));
        } else if text_entered {
            lines.push(Spans::from(vec![bold("   [Esc]"), plain("-clear")]));
        } else {
            lines.push(Spans::default());
        }
    } else {
        lines.push(Spans::from(vec![bold("   [Esc]"), plain("-cancel")]));
    }

    if input_mode == InputMode::Normal {
        lines.push(Spans::from(vec![bold("     [Q]"), plain("-quit")]));
    } else {
        lines.push(Spans::default());
    }

    Paragraph::new(lines)
}

pub fn render_input<B: Backend>(f: &mut Frame<B>, app: &App, cell_input: Rect) {
    let mut input_block = Block::default()
        .borders(Borders::ALL);
    if !app.recipient.valid {
        input_block = input_block.title(" Select a recipient. ");
    } else {
        let send_to = Spans::from(format!(" sending to: {} - {} ",
            app.recipient.peer.name, app.recipient.peer.address.to_string()));
        input_block = input_block.title(send_to);
    }

    let line = app.input.split('\n').last().unwrap_or("");
    let start = line.len().saturating_sub(cell_input.width as usize - 3);
    let end = min(line.len(), start + cell_input.width as usize - 3);
    let line = line.get(start..end).unwrap_or("<range error>");

    let input = Paragraph::new(line)
        .style(match app.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Editing => Style::default().fg(Color::Yellow),
        })
        .block(input_block);

    f.render_widget(input, cell_input);
    match app.input_mode {
        InputMode::Normal => {}

        InputMode::Editing => {
            // Make the cursor visible and ask tui-rs to put it at the specified
            // coordinates after rendering
            f.set_cursor(
                // Put cursor past the end of the input text
                cell_input.x + line.width() as u16 + 1,
                // Move one line down, from the border to the input line
                cell_input.y + 1,
            )
        }
    }
}

pub fn ui_info<'a>(app: &'a App) -> Paragraph<'a> {
    Paragraph::new(vec![
        Spans::from("computer name:"),
        Spans::from(bold(&app.lan.local_name)),
        Spans::from("internal address:"),
        if app.lan.local_addr.len() > 0 {
            Spans::from(bold(&app.lan.local_addr))
        } else {
            Spans::from(faded("(not connected)"))
        },
    ])
}

fn message_heading(message: &Message) -> Spans<'static> {
    let mut heading = vec![];
    match message.direction {
        MessageType::Sent => {
            heading.push(bold("→"));
            heading.push(plain(" to     "));
        }
        MessageType::Sending => {
            heading.push(bold("→"));
            heading.push(plain(" ...    "));
        }
        MessageType::SendFailed => {
            heading.push(bold("→ failed "));
        }
        MessageType::Received => {
            heading.push(bold("←"));
            heading.push(plain(" from   "));
        }
    }

    heading.push(bold(message.name.clone()));

    let len = 16usize.saturating_sub(message.name.len());
    heading.push(plain(format!("{:_<len$} {}", "", message.timestamp, len=len)));

    let heading_color = match message.direction {
        MessageType::Sent => Color::Yellow,
        MessageType::Sending => Color::DarkGray,
        MessageType::SendFailed => Color::Red,
        MessageType::Received => Color::LightCyan,
    };

    for span in &mut heading {
        span.style = span.style.fg(heading_color);
    }

    Spans::from(heading)
}

pub fn ui_messages(app: &App, area: Rect) -> Paragraph<'static> {
    let view_width = area.width - 2;
    let view_height = area.height - 2;

    let mut focus_y = None;

    let mut lines: Vec<Spans<'static>> = vec![];
    for (i, message) in app.messages.iter().enumerate() {
        let mut body_style = Style::default();
        if Some(i as u16) == app.message_highlight {
            body_style = body_style.add_modifier(Modifier::REVERSED);

            focus_y = Some(lines.len() as u16);
        }

        lines.push(message_heading(message.clone()));
        for r in wrap(&message.content, view_width as usize) {
            lines.push(Spans::from(Span::styled(r.into_owned(), body_style)));
        }
    }

    while lines.len() < view_height as usize {
        lines.insert(0, Spans::default());
    }

    let lowest = (lines.len() as u16).saturating_sub(view_height);
    let y = if let Some(r) = focus_y {
        min(r.saturating_sub((view_height / 2).saturating_sub(1)), lowest)
    } else {
        lowest
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(if app.message_highlight.is_some() {
            Style::default().fg(Color::LightCyan)
        } else {
            Style::default()
        });

    Paragraph::new(lines)
        .block(block)
        .scroll((y, 0))
}

pub fn ui_status<'a>(app: &'a App) -> Paragraph<'a> {
    Paragraph::new(Spans::from(vec![
        Span::styled(
            &app.status,
            Style::default().fg(Color::Gray).bg(Color::Red)
        ).into(),
        plain(" "),
    ])).alignment(Alignment::Right)
}
