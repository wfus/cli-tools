use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::app::{App, ModelFilter};
use super::widgets::{minute_chart::draw_minute_chart, request_feed::draw_request_feed, stats_panel::draw_stats_panel, summary_bar::draw_summary_bar};

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(3),  // Header
                Constraint::Min(15),    // Main content
                Constraint::Length(3),  // Summary
                Constraint::Length(3),  // Help
            ]
            .as_ref(),
        )
        .split(f.size());

    draw_header(f, chunks[0], app);
    draw_main_content(f, chunks[1], app);
    draw_summary_bar(f, chunks[2], app);
    draw_help(f, chunks[3]);
}

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let model_text = match &app.model_filter {
        ModelFilter::All => "All Models".to_string(),
        ModelFilter::Specific(m) => m.to_string(),
    };

    let header_text = vec![
        Span::raw("Model: "),
        Span::styled(model_text, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" ▼ | Last Update: "),
        Span::raw(app.last_update.format("%H:%M:%S").to_string()),
        Span::raw(" | Auto-refresh: 5s"),
    ];

    let header = Paragraph::new(Line::from(header_text))
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .title(" Claude Usage Dashboard ")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Cyan)),
        );

    f.render_widget(header, area);
}

fn draw_main_content(f: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(65),  // Left side (chart + feed)
                Constraint::Percentage(35),  // Right side (stats)
            ]
            .as_ref(),
        )
        .split(area);

    // Left side - chart and feed
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(12),  // Chart
                Constraint::Min(5),      // Feed
            ]
            .as_ref(),
        )
        .split(chunks[0]);

    draw_minute_chart(f, left_chunks[0], app);
    draw_request_feed(f, left_chunks[1], app);

    // Right side - stats
    draw_stats_panel(f, chunks[1], app);
}

fn draw_help(f: &mut Frame, area: Rect) {
    let help_text = vec![
        Span::raw("["),
        Span::styled("q", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw("]uit ["),
        Span::styled("m", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw("]odel ["),
        Span::styled("t", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw("]ime-range ["),
        Span::styled("↑↓", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw("] scroll ["),
        Span::styled("p", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw("]ause ["),
        Span::styled("h", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw("]elp"),
    ];

    let help = Paragraph::new(Line::from(help_text))
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::TOP))
        .alignment(Alignment::Center);

    f.render_widget(help, area);
}