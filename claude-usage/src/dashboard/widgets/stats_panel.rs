use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::dashboard::app::{App, ModelFilter};
use crate::dashboard::data::TimeRangeStats;

pub fn draw_stats_panel(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10),  // Current hour
            Constraint::Length(10),  // Last 5 hours
            Constraint::Length(10),  // Last 24 hours
            Constraint::Length(10),  // Last 2 days
            Constraint::Length(10),  // Last 7 days
            Constraint::Min(1),      // Remaining space
        ].as_ref())
        .split(area);

    let model_filter = match &app.model_filter {
        ModelFilter::All => None,
        ModelFilter::Specific(m) => Some(m),
    };

    let current_stats = app.rolling_window.get_current_hour_stats(model_filter);
    let stats_5h = app.rolling_window.get_5h_stats(model_filter);
    let stats_24h = app.rolling_window.get_24h_stats(model_filter);
    let stats_2d = app.rolling_window.get_2d_stats(model_filter);
    let stats_7d = app.rolling_window.get_7d_stats(model_filter);

    draw_stats_widget(f, chunks[0], &current_stats, " Current Hour Stats ");
    draw_stats_widget(f, chunks[1], &stats_5h, " Last 5 Hours ");
    draw_stats_widget(f, chunks[2], &stats_24h, " Last 24 Hours ");
    draw_stats_widget(f, chunks[3], &stats_2d, " Last 2 Days ");
    draw_stats_widget(f, chunks[4], &stats_7d, " Last 7 Days ");
}

fn draw_stats_widget(f: &mut Frame, area: Rect, stats: &TimeRangeStats, title: &str) {
    let mut lines = vec![
        Line::from(vec![
            Span::raw("Requests: "),
            Span::styled(
                format!("{}", stats.requests),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("Tokens: "),
            Span::styled(
                format_tokens(stats.tokens),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("Cost: "),
            Span::styled(
                format!("${:.2}", stats.cost),
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled("By Model:", Style::default().add_modifier(Modifier::UNDERLINED))),
    ];

    // Add model breakdown
    let mut model_entries: Vec<_> = stats.model_costs.iter().collect();
    model_entries.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());

    for (model, cost) in model_entries {
        let color = match model.as_str() {
            "opus" => Color::Magenta,
            "sonnet" => Color::Yellow,
            "haiku" => Color::Green,
            _ => Color::White,
        };

        lines.push(Line::from(vec![
            Span::raw(" ▪ "),
            Span::styled(
                format!("{:<7}", capitalize(model)),
                Style::default().fg(color),
            ),
            Span::styled(
                format!("${:.2}", cost),
                Style::default().fg(Color::Green),
            ),
        ]));
    }

    let widget = Paragraph::new(lines)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL),
        )
        .alignment(Alignment::Left);

    f.render_widget(widget, area);
}

fn format_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        format!("{}", tokens)
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
    }
}