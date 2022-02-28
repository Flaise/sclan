use tui::widgets::Paragraph;
use tui::text::{Spans, Span};
use tui::style::{Style, Modifier};

fn ui_scrollbar(width: u16, position: u16, count: u16) -> Spans<'static> {
    let mut bar = vec![];
    for i in 0..width {
        let span = if i == position * count / width {
            Span::raw("▓")
        } else {
            Span::raw("░")
        };
        bar.push(span);
    }
    Spans::from(bar)
}

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
        if let Some(count) = (max_options as usize).checked_sub(lines.len() - 1) {
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
