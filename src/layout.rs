use tui::backend::Backend;
use tui::layout::{Constraint, Direction, Layout, Alignment, Rect};
use tui::Frame;
use crate::App;
use crate::render::{ui_scrolling_list, render_input, ui_instructions, ui_info, ui_messages,
                    ui_status};

struct Cells {
    cell_info: Rect,
    cell_peers: Rect,
    cell_instructions: Rect,
    cell_input: Rect,
    cell_messages: Rect,
    cell_status: Rect,
}

fn calc_layout(base: Rect) -> Cells {
    let horiz = Layout::default()
        .direction(Direction::Horizontal)
        .vertical_margin(1)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(18),
            Constraint::Min(8),
        ].as_ref())
        .split(base);

    let side = Layout::default()
        .constraints([
            Constraint::Length(7),
            Constraint::Min(3),
            Constraint::Length(8),
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
            Constraint::Length(1),
            Constraint::Length(3),
        ].as_ref())
        .split(horiz[2]);

    let cell_messages = vert[0];
    let cell_status = vert[1];
    let cell_input = vert[2];

    Cells {cell_info, cell_peers, cell_instructions, cell_input, cell_messages, cell_status}
}

pub fn ui<B: Backend>(frame: &mut Frame<B>, app: &App) {
    let Cells {cell_info, cell_peers, cell_instructions, cell_input, cell_messages, cell_status} =
        calc_layout(frame.size());

    frame.render_widget(ui_info(app).alignment(Alignment::Right), cell_info);

    let options = app.lan.peers.iter().map(|peer| peer.name.clone()).collect::<Vec<_>>();
    frame.render_widget(ui_scrolling_list(
        cell_peers, "network:", &app.recipient.peer.name, &options
    ).alignment(Alignment::Right), cell_peers);

    frame.render_widget(ui_instructions(
        app.input_mode, app.recipient.valid, app.input.trim().len() > 0, app.messages.len() > 0,
        app.message_highlight.is_some()
    ), cell_instructions);

    frame.render_widget(ui_status(app), cell_status);

    render_input(frame, app, cell_input);

    frame.render_widget(ui_messages(app, cell_messages), cell_messages);
}
