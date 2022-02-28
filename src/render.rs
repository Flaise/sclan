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

    for (i, label) in options.iter().enumerate() {
        if i >= max_options as usize {
            break;
        }

        let span = if label == selection {
            // let label = label.clone();
            let label = format!("< {} >", label);
            Span::styled(label, Style::default().add_modifier(Modifier::REVERSED))
        } else {
            Span::raw(label.clone())
        };
        lines.push(Spans::from(span));
    }

    if options.len() == 0 {
        lines.push(Spans::from(" (none) "));
        //     Span::styled(" (none) ", Style::default().add_modifier(Modifier::ITALIC))
        // ));
    }

    while lines.len() < max_options as usize {
        lines.push(Spans::default());
    }

    if options.len() > max_options as usize {
        // TODO
        lines.push(ui_scrollbar(18, 11, 18));
    } else {
        lines.push(Spans::default());
    }

    Paragraph::new(lines)
}
