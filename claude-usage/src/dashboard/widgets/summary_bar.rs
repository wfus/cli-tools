use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::dashboard::app::App;

pub fn draw_summary_bar(f: &mut Frame, area: Rect, app: &App) {
    let (total_cost, model_costs) = app.rolling_window.get_24h_stats();

    let mut spans = vec![
        Span::raw("Total: "),
        Span::styled(
            format!("${:.2}", total_cost),
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        ),
    ];

    // Add model breakdowns
    let mut model_entries: Vec<_> = model_costs.into_iter().collect();
    model_entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    for (model, cost) in model_entries {
        let color = match model.as_str() {
            "opus" => Color::Magenta,
            "sonnet" => Color::Yellow,
            "haiku" => Color::Green,
            _ => Color::White,
        };

        spans.push(Span::raw(" │ "));
        spans.push(Span::styled(
            format!("{}: ", capitalize(&model)),
            Style::default().fg(color),
        ));
        spans.push(Span::styled(
            format!("${:.2}", cost),
            Style::default().fg(Color::Green),
        ));
    }

    let summary = Paragraph::new(Line::from(spans))
        .block(
            Block::default()
                .title(" 24-Hour Summary ")
                .borders(Borders::ALL),
        )
        .alignment(Alignment::Center);

    f.render_widget(summary, area);
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
    }
}