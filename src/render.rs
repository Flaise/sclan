use tui::widgets::Paragraph;
use tui::text::{Spans, Span};
use tui::style::{Style, Modifier};

fn ui_scrollbar(width: u16, position: u16, count: u16) -> Spans<'static> {
    let mut bar = vec![];
    for i in 0..width {
        let span = if i == 5 {
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

    for (i, label) in options.iter().enumerate() {
        if i >= max_options as usize {
            break;
        }

        let span = if label == selection {
            // let label = label.clone();
            // Span::styled(label, Style::default().add_modifier(Modifier::REVERSED))

            let label = format!("< {} >", label);
            Span::styled(label, Style::default().add_modifier(Modifier::BOLD))
        } else {
            Span::raw(label.clone())
        };
        lines.push(Spans::from(span));
    }

    while lines.len() < max_options as usize {
        lines.push(Spans::default());
    }

    if options.len() > max_options as usize {
        lines.push(ui_scrollbar(18, 5, 18));
    } else {
        lines.push(Spans::default());
    }

    Paragraph::new(lines)
}
