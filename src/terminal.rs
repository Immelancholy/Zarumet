use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::DefaultTerminal;

/// Initialize the terminal for the application
pub fn init_terminal() -> color_eyre::Result<DefaultTerminal> {
    // Initialize terminal with explicit crossterm configuration for full control
    execute!(std::io::stdout(), EnterAlternateScreen)?;
    enable_raw_mode()?;

    let terminal =
        ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(std::io::stdout()))?;
    
    Ok(terminal)
}

/// Restore the terminal to its original state
pub fn restore_terminal() -> color_eyre::Result<()> {
    // Restore terminal
    execute!(std::io::stdout(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}