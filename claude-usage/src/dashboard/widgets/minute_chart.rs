use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    symbols,
    text::Span,
    widgets::{Axis, BarChart, Block, Borders, Chart, Dataset},
    Frame,
};

use crate::dashboard::app::{App, ChartType, ModelFilter};

pub fn draw_minute_chart(f: &mut Frame, area: Rect, app: &App) {
    match app.chart_type {
        ChartType::Bar => draw_bar_chart(f, area, app),
        ChartType::Line => draw_line_chart(f, area, app),
    }
}

fn draw_bar_chart(f: &mut Frame, area: Rect, app: &App) {
    let model_filter = match &app.model_filter {
        ModelFilter::All => None,
        ModelFilter::Specific(m) => Some(m),
    };

    let minute_costs = app.rolling_window.get_minute_costs(model_filter);
    
    // Create bars for the last N minutes
    let now = chrono::Utc::now();
    let window_minutes = app.time_range.minutes();
    
    // Group data into buckets (e.g., 5-minute buckets for better visibility)
    let bucket_size = if window_minutes <= 60 { 1 } else if window_minutes <= 360 { 5 } else { 10 };
    let num_buckets = window_minutes / bucket_size;
    
    let mut buckets: Vec<(String, f64)> = Vec::new();
    
    // Initialize buckets
    for i in 0..num_buckets {
        let minutes_ago = i * bucket_size;
        let label = if minutes_ago == 0 {
            "now".to_string()
        } else if minutes_ago % 60 == 0 {
            format!("-{}h", minutes_ago / 60)
        } else if minutes_ago % 10 == 0 {
            format!("-{}", minutes_ago)
        } else {
            String::new()
        };
        buckets.push((label, 0.0));
    }
    
    // Fill buckets with cost data
    for (timestamp, cost) in minute_costs {
        let minutes_ago = (now - timestamp).num_minutes() as usize;
        let bucket_idx = minutes_ago / bucket_size;
        if bucket_idx < buckets.len() {
            buckets[bucket_idx].1 += cost;
        }
    }
    
    // Reverse so newest is on the right
    buckets.reverse();
    
    // Calculate max for scaling
    let max_cost = buckets.iter().map(|(_, cost)| *cost).fold(0.0, f64::max);
    
    // Create bar chart data
    let bar_data: Vec<(&str, u64)> = buckets
        .iter()
        .map(|(label, cost)| (label.as_str(), (*cost * 1000.0) as u64)) // Scale to millicents for integer display
        .collect();

    let bar_chart = BarChart::default()
        .block(
            Block::default()
                .title(format!(" Rolling {}-Minute Usage (${:.2} max) ", window_minutes, max_cost))
                .borders(Borders::ALL),
        )
        .data(&bar_data)
        .bar_width(3)
        .bar_gap(1)
        .bar_style(Style::default().fg(Color::Cyan))
        .value_style(
            Style::default()
                .fg(Color::White)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(bar_chart, area);
}

fn draw_line_chart(f: &mut Frame, area: Rect, app: &App) {
    let model_filter = match &app.model_filter {
        ModelFilter::All => None,
        ModelFilter::Specific(m) => Some(m),
    };

    let minute_costs = app.rolling_window.get_minute_costs(model_filter);
    
    // Get time window parameters
    let now = chrono::Utc::now();
    let window_minutes = app.time_range.minutes();
    
    // Group data into buckets for smoothing
    let bucket_size = if window_minutes <= 60 { 1 } else if window_minutes <= 360 { 5 } else { 10 };
    let num_buckets = window_minutes / bucket_size;
    
    let mut data_points: Vec<(f64, f64)> = Vec::new();
    
    // Initialize buckets with zeros
    let mut buckets: Vec<f64> = vec![0.0; num_buckets];
    
    // Fill buckets with cost data
    for (timestamp, cost) in minute_costs {
        let minutes_ago = (now - timestamp).num_minutes() as usize;
        let bucket_idx = minutes_ago / bucket_size;
        if bucket_idx < buckets.len() {
            buckets[bucket_idx] += cost;
        }
    }
    
    // Create data points (x: time index, y: cost)
    for (i, cost) in buckets.iter().enumerate() {
        let x = (num_buckets - 1 - i) as f64; // Reverse so newest is on the right
        data_points.push((x, *cost));
    }
    
    // Calculate bounds
    let max_cost = buckets.iter().fold(0.0, |max, &cost| if cost > max { cost } else { max });
    let y_max = if max_cost > 0.0 { max_cost * 1.1 } else { 0.1 }; // Add 10% padding
    
    // Create x-axis labels
    let x_labels: Vec<Span> = (0..num_buckets)
        .step_by((num_buckets / 10).max(1))
        .map(|i| {
            let minutes_ago = (num_buckets - 1 - i) * bucket_size;
            if minutes_ago == 0 {
                Span::raw("now")
            } else if minutes_ago >= 60 && minutes_ago % 60 == 0 {
                Span::raw(format!("-{}h", minutes_ago / 60))
            } else {
                Span::raw(format!("-{}m", minutes_ago))
            }
        })
        .collect();
    
    // Create y-axis labels
    let y_labels: Vec<Span> = (0..=5)
        .map(|i| {
            let value = y_max * (i as f64) / 5.0;
            Span::raw(format!("${:.2}", value))
        })
        .collect();
    
    let datasets = vec![
        Dataset::default()
            .name("Cost")
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(Color::Cyan))
            .data(&data_points),
    ];
    
    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .title(format!(" Rolling {}-Minute Usage ", window_minutes))
                .borders(Borders::ALL),
        )
        .x_axis(
            Axis::default()
                .title("Time")
                .style(Style::default().fg(Color::Gray))
                .labels(x_labels)
                .bounds([0.0, (num_buckets - 1) as f64]),
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