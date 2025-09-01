use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::{error::Error, io};

mod app;
mod config;
mod encryption;
mod note;
mod ui;

use app::App;
use config::Config;

fn main() -> Result<(), Box<dyn Error>> {
        let config = Config::load()?;
    
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(&config)?;
    let res = run_app(&mut terminal, &mut app, &config);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    config: &Config,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app, config))?;

        if event::poll(std::time::Duration::from_millis(config.behavior.ui_timeout_ms))? {
            if let Event::Key(key) = event::read()? {
                app.handle_input(key, config)?;
                if app.should_quit {
                    return Ok(());
                }
            }

            // batch process paste spam so the ui doesn't shit itself
            let mut events_processed = 1;
            let max_events = config.behavior.max_events_per_frame;
            
            while events_processed < max_events 
                && event::poll(std::time::Duration::from_millis(0))? {
                if let Event::Key(key) = event::read()? {
                    app.handle_input(key, config)?;
                    if app.should_quit {
                        return Ok(());
                    }
                    events_processed += 1;
                }
            }
        }
    }
}