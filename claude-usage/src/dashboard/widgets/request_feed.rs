use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::dashboard::app::App;

pub fn draw_request_feed(f: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .request_feed
        .iter()
        .skip(app.feed_scroll)
        .take(area.height as usize - 2) // Account for borders
        .map(|request| {
            let model_color = match request.model.family() {
                "opus" => Color::Magenta,
                "sonnet" => Color::Yellow,
                "haiku" => Color::Green,
                _ => Color::White,
            };

            let line = vec![
                Span::raw("["),
                Span::raw(request.timestamp.format("%H:%M:%S").to_string()),
                Span::raw("] "),
                Span::styled(
                    format!("{:<8}", request.model.family()),
                    Style::default().fg(model_color).add_modifier(Modifier::BOLD),
                ),
                Span::raw(" │ "),
                Span::raw(format!("{:>5} in", request.input_tokens)),
                Span::raw(" / "),
                Span::raw(format!("{:>5} out", request.output_tokens)),
                Span::raw(" │ Cache: "),
                Span::raw(format!("{:>4}", request.cache_tokens)),
                Span::raw(" │ "),
                Span::styled(
                    format!("${:.2}", request.cost),
                    Style::default().fg(Color::Green),
                ),
            ];

            ListItem::new(Line::from(line))
        })
        .collect();

    let title = if app.feed_paused {
        " Live Request Feed [PAUSED] "
    } else {
        " Live Request Feed "
    };

    let feed = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .style(if app.feed_paused {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default()
                }),
        );

    f.render_widget(feed, area);
}