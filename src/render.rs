use std::cmp::min;
use tui::{backend::Backend, layout::{Rect}, Frame};
use tui::widgets::{Paragraph, List, ListItem, Block, Borders};
use tui::text::{Spans, Span};
use tui::style::{Style, Modifier, Color};
use unicode_width::UnicodeWidthStr;
use crate::App;
use crate::data::InputMode;

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
        let span = Span::styled(label, Style::default().add_modifier(Modifier::REVERSED));
        lines.push(Spans::from(span));

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
                       text_entered: bool) -> Paragraph<'static> {
    let mut lines = vec![];

    lines.push(Spans::from(vec![
        Span::styled("   [Tab]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("-recipient"),
    ]));
    
    if !recipient_valid {
        lines.push(Spans::default());
    } else if input_mode == InputMode::Normal {
        lines.push(Spans::from(vec![
            Span::styled(" [Enter]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("-write"),
        ]));
    } else if text_entered {
        lines.push(Spans::from(vec![
            Span::styled(" [Enter]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("-send"),
        ]));
    } else {
        lines.push(Spans::default());
    }

    if input_mode == InputMode::Normal {
        if text_entered {
            lines.push(Spans::from(vec![
                Span::styled("   [Esc]", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw("-clear"),
            ]));
        } else {
            lines.push(Spans::default());
        }
    } else {
        lines.push(Spans::from(vec![
            Span::styled("   [Esc]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("-cancel"),
        ]));
    }

    lines.push(Spans::from(vec![
        Span::styled(" [Alt+V]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("-paste"),
    ]));

    if input_mode == InputMode::Normal {
        lines.push(Spans::from(vec![
            Span::styled("     [Q]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("-quit"),
        ]));
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
            // Make the cursor visible and ask tui-rs to put it at the specified
            // coordinates after rendering
            f.set_cursor(
                // Put cursor past the end of the input text
                cell_input.x + app.input.width() as u16 + 1,
                // Move one line down, from the border to the input line
                cell_input.y + 1,
            )
        }
    }
}

pub fn ui_info(app: &App) -> Paragraph<'static> {
    Paragraph::new(vec![
        Spans::from("computer name:"),
        Spans::from(Span::styled(
            app.lan.local_name.clone(),
            Style::default().add_modifier(Modifier::BOLD)
        )),
    ])
}

pub fn ui_messages(app: &App) -> List {
    let messages: Vec<ListItem> = app
        .messages
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let content = vec![Spans::from(Span::raw(format!("{}: {}", i, m)))];
            ListItem::new(content)
        })
        .collect();
    List::new(messages)
        .block(Block::default()
        .borders(Borders::ALL))
}
