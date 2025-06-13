use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::dashboard::app::{App, ModelFilter};

pub fn draw_stats_panel(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),  // Current hour
            Constraint::Length(8),  // Last 5 hours
            Constraint::Length(8),  // Last 24 hours
            Constraint::Min(1),     // Remaining space
        ].as_ref())
        .split(area);

    draw_current_hour_stats(f, chunks[0], app);
    draw_5h_stats(f, chunks[1], app);
    draw_24h_stats(f, chunks[2], app);
}

fn draw_current_hour_stats(f: &mut Frame, area: Rect, app: &App) {
    let model_filter = match &app.model_filter {
        ModelFilter::All => None,
        ModelFilter::Specific(m) => Some(m),
    };

    let (requests, tokens, cost, model_costs) = app.rolling_window.get_current_hour_stats(model_filter);

    let mut lines = vec![
        Line::from(vec![
            Span::raw("Requests: "),
            Span::styled(
                format!("{}", requests),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("Tokens: "),
            Span::styled(
                format_tokens(tokens),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("Cost: "),
            Span::styled(
                format!("${:.2}", cost),
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled("By Model:", Style::default().add_modifier(Modifier::UNDERLINED))),
    ];

    // Add model breakdown
    let mut model_entries: Vec<_> = model_costs.into_iter().collect();
    model_entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

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
                format!("{:<7}", capitalize(&model)),
                Style::default().fg(color),
            ),
            Span::styled(
                format!("${:.2}", cost),
                Style::default().fg(Color::Green),
            ),
        ]));
    }

    let stats = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Current Hour Stats ")
                .borders(Borders::ALL),
        )
        .alignment(Alignment::Left);

    f.render_widget(stats, area);
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

fn draw_5h_stats(f: &mut Frame, area: Rect, app: &App) {
    let model_filter = match &app.model_filter {
        ModelFilter::All => None,
        ModelFilter::Specific(m) => Some(m),
    };

    let (requests, tokens, cost, model_costs) = app.rolling_window.get_5h_stats(model_filter);

    let mut lines = vec![
        Line::from(vec![
            Span::raw("Requests: "),
            Span::styled(
                format!("{}", requests),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("Tokens: "),
            Span::styled(
                format_tokens(tokens),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("Cost: "),
            Span::styled(
                format!("${:.2}", cost),
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled("By Model:", Style::default().add_modifier(Modifier::UNDERLINED))),
    ];

    // Add model breakdown
    let mut model_entries: Vec<_> = model_costs.into_iter().collect();
    model_entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

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
                format!("{:<7}", capitalize(&model)),
                Style::default().fg(color),
            ),
            Span::styled(
                format!("${:.2}", cost),
                Style::default().fg(Color::Green),
            ),
        ]));
    }

    let stats = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Last 5 Hours ")
                .borders(Borders::ALL),
        )
        .alignment(Alignment::Left);

    f.render_widget(stats, area);
}

fn draw_24h_stats(f: &mut Frame, area: Rect, app: &App) {
    let model_filter = match &app.model_filter {
        ModelFilter::All => None,
        ModelFilter::Specific(m) => Some(m),
    };

    let (requests, tokens, cost, model_costs) = app.rolling_window.get_24h_stats(model_filter);

    let mut lines = vec![
        Line::from(vec![
            Span::raw("Requests: "),
            Span::styled(
                format!("{}", requests),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("Tokens: "),
            Span::styled(
                format_tokens(tokens),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("Cost: "),
            Span::styled(
                format!("${:.2}", cost),
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled("By Model:", Style::default().add_modifier(Modifier::UNDERLINED))),
    ];

    // Add model breakdown
    let mut model_entries: Vec<_> = model_costs.into_iter().collect();
    model_entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

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
                format!("{:<7}", capitalize(&model)),
                Style::default().fg(color),
            ),
            Span::styled(
                format!("${:.2}", cost),
                Style::default().fg(Color::Green),
            ),
        ]));
    }

    let stats = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Last 24 Hours ")
                .borders(Borders::ALL),
        )
        .alignment(Alignment::Left);

    f.render_widget(stats, area);
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
    }
}