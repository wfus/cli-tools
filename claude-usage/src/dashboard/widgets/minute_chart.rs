use ratatui::{
    layout::Rect,
    style::{Color, Style},
    symbols,
    text::Span,
    widgets::{Axis, Block, Borders, Chart, Dataset},
    Frame,
};

use crate::dashboard::app::{App, ModelFilter};

pub fn draw_minute_chart(f: &mut Frame, area: Rect, app: &App) {
    let model_filter = match &app.model_filter {
        ModelFilter::All => None,
        ModelFilter::Specific(m) => Some(m),
    };

    let minute_costs = app.rolling_window.get_minute_costs(model_filter);
    
    // Convert to chart data points
    let now = chrono::Utc::now();
    let data: Vec<(f64, f64)> = minute_costs
        .iter()
        .map(|(timestamp, cost)| {
            let minutes_ago = (now - *timestamp).num_minutes() as f64;
            (-minutes_ago, *cost)
        })
        .collect();

    // Calculate Y-axis bounds
    let max_cost = data.iter().map(|(_, cost)| *cost).fold(0.0, f64::max);
    let y_max = if max_cost > 0.0 {
        (max_cost * 1.2 * 100.0).ceil() / 100.0 // Round up to nearest cent with 20% padding
    } else {
        0.50 // Default to $0.50 if no data
    };

    let datasets = vec![Dataset::default()
        .name("Cost")
        .marker(symbols::Marker::Braille)
        .style(Style::default().fg(Color::Cyan))
        .data(&data)];

    let x_labels = vec![
        Span::raw(format!("-{}", app.time_range.minutes())),
        Span::raw("-30"),
        Span::raw("-10"),
        Span::raw("now"),
    ];

    let y_labels = vec![
        Span::raw("$0.00"),
        Span::raw(format!("${:.2}", y_max / 2.0)),
        Span::raw(format!("${:.2}", y_max)),
    ];

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .title(format!(" Rolling {}-Minute Usage ", app.time_range.minutes()))
                .borders(Borders::ALL),
        )
        .x_axis(
            Axis::default()
                .title("Time")
                .style(Style::default().fg(Color::Gray))
                .labels(x_labels)
                .bounds([-(app.time_range.minutes() as f64), 0.0]),
        )
        .y_axis(
            Axis::default()
                .title("Cost ($)")
                .style(Style::default().fg(Color::Gray))
                .labels(y_labels)
                .bounds([0.0, y_max]),
        );

    f.render_widget(chart, area);
}