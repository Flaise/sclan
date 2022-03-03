use std::borrow::Cow;
use std::cmp::min;
use std::iter::Iterator;
use tui::{backend::Backend, layout::Rect, Frame};
use tui::widgets::{Paragraph, Block, Borders};
use tui::text::{Spans, Span};
use tui::style::{Style, Modifier, Color};
use unicode_width::UnicodeWidthStr;
use textwrap::core::{Fragment, Word};
use textwrap::wrap_algorithms::wrap_first_fit;
use textwrap::WordSeparator::AsciiSpace;
use textwrap::wrap;
use crate::App;
use crate::data::{InputMode, MessageDirection, Message};

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

pub fn ui_scrolling_list(max_options: u16, title: &str, selection: &str, options: &[String])
-> Paragraph<'static> {

    let mut lines = vec![
        Spans::from(title.to_string()),
    ];

    if let Some(index) = options.iter().position(|a| a == selection) {
        let label = format!("→ {}", options[index]);
        lines.push(Spans::from(reversed(label.clone())));

        for label in options.iter().skip(index + 1).take(max_options as usize - 1) {
            lines.push(Spans::from(label.clone()));
        }
        if let Some(count) = min(options.len(), max_options as usize).checked_sub(lines.len() - 1) {
            for label in options.iter().take(count) {
                lines.push(Spans::from(label.clone()));
            }
        }
    } else {
        for label in options.iter().take(max_options as usize) {
            lines.push(Spans::from(label.clone()));
        }
    }

    if options.len() == 0 {
        lines.push(Spans::from(" (none) "));
    }

    while lines.len() < max_options as usize {
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

    if input_mode == InputMode::Normal && output_displayed {
        lines.push(Spans::from(vec![bold(" [↑] [↓]"), plain("-message")]));
    } else {
        lines.push(Spans::default());
    }
    lines.push(Spans::default());

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

    lines.push(Spans::from(vec![bold(" [Alt+V]"), plain("-paste")]));

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
        let send_to = Spans::from(format!(" sending to: {} ", app.recipient.name));
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

pub fn ui_info(app: &App) -> Paragraph<'static> {
    Paragraph::new(vec![
        Spans::from("computer name:"),
        Spans::from(bold(app.lan.local_name.clone())),
    ])
}

#[derive(Debug)]
struct SpanFragment<'a> {
    word: Word<'a>,
    style: Style,
}

impl Fragment for SpanFragment<'_> {
    fn width(&self) -> f64 {
        self.word.width()
    }
    fn whitespace_width(&self) -> f64 {
        self.word.whitespace_width()
    }
    fn penalty_width(&self) -> f64 {
        self.word.penalty_width()
    }
}

fn message_heading(message: &Message) -> Spans<'static> {
    let mut heading = vec![];
    if message.direction == MessageDirection::Sent {
        heading.push(bold("→"));
        heading.push(plain(" to   "));
    } else {
        heading.push(bold("←"));
        heading.push(plain(" from "));
    }
    // heading.push(bold(format!("{:.<16}", message.name)));

    heading.push(bold(message.name.clone()));
    for _ in message.name.len()..16 {
        heading.push(plain("_"));
    }

    heading.push(plain(format!(" {}", message.timestamp)));

    let heading_color = if message.direction == MessageDirection::Sent {
        Color::Yellow
    } else {
        Color::LightCyan
    };
    for span in &mut heading {
        span.style = span.style.fg(heading_color);
    }

    Spans::from(heading)
}

fn wrap_line(line: &Spans, width: u16) -> Vec<Spans<'static>> {
//impl Iterator<Item = Spans<'static>> {
    let mut fragments = vec![];

    for span in &line.0 {
        // The disadvantage of doing it this way is that spans with two different styles won't
        // remain joined as one word at the wrap boundary. Not sure what the better way to do
        // this would be.

        for word in AsciiSpace.find_words(&span.content) {
            fragments.push(SpanFragment {
                word,
                style: span.style,
            });
        }
    }

    let group = wrap_first_fit(&fragments, &[width as f64]);
    group.iter().map(|row| {
        Spans::from(
            row.iter().map(|fragment| {
                Span::styled(
                    format!("{}{}", fragment.word.word, fragment.word.whitespace),
                    fragment.style
                )
            }).collect::<Vec<Span>>()
        )
    }).collect()
}

pub fn ui_messages(app: &App, area: Rect) -> Paragraph<'static> {
    let view_width = area.width - 2;
    let view_height = area.height - 2;

    let mut lines: Vec<Spans<'static>> = vec![];
    for (i, message) in app.messages.iter().enumerate() {
        lines.push(message_heading(message.clone()));

        let style = if Some(i as u16) == app.message_highlight {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };

        for r in wrap(&message.content, view_width as usize) {
            lines.push(Spans::from(Span::styled(r.into_owned(), style)));
        }
    }

    while lines.len() < view_height as usize {
        lines.insert(0, Spans::default());
    }

    // let prefer_y = view_height / 2;

    let lowest = (lines.len() as u16).saturating_sub(view_height);
    let y = lowest;//min(999, lowest);

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
