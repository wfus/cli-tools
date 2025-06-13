pub mod app;
pub mod data;
pub mod events;
pub mod runner;
pub mod ui;
pub mod widgets;

// Re-export the main function
pub use runner::run_dashboard;