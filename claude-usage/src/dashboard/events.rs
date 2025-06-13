use crossterm::event::{KeyCode, KeyEvent};

use super::app::App;

pub fn handle_key_event(key: KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Char('m') => {
            app.cycle_model_filter();
        }
        KeyCode::Char('t') => {
            app.cycle_time_range();
        }
        KeyCode::Char('c') => {
            app.toggle_chart_type();
        }
        KeyCode::Up => {
            app.scroll_feed_up();
        }
        KeyCode::Down => {
            app.scroll_feed_down();
        }
        KeyCode::Char('p') => {
            app.toggle_feed_pause();
        }
        KeyCode::Char('r') => {
            // Force refresh - will be handled in the next tick
        }
        _ => {}
    }
}