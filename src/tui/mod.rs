//! Interactive TUI for skill management (requires `tui` feature)

mod skill_browser;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::io;
use std::time::Duration;

use crate::config::Config;
use crate::skill::Skill;

/// Which view is currently active
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
    SkillBrowser,
    GraphView,
    InstallDashboard,
    HealthPanel,
}

impl ActiveView {
    /// Get the next view in the cycle
    fn next(self) -> Self {
        match self {
            ActiveView::SkillBrowser => ActiveView::GraphView,
            ActiveView::GraphView => ActiveView::InstallDashboard,
            ActiveView::InstallDashboard => ActiveView::HealthPanel,
            ActiveView::HealthPanel => ActiveView::SkillBrowser,
        }
    }

    /// Get the previous view in the cycle
    fn prev(self) -> Self {
        match self {
            ActiveView::SkillBrowser => ActiveView::HealthPanel,
            ActiveView::GraphView => ActiveView::SkillBrowser,
            ActiveView::InstallDashboard => ActiveView::GraphView,
            ActiveView::HealthPanel => ActiveView::InstallDashboard,
        }
    }

    /// Get the display name of the view
    fn name(self) -> &'static str {
        match self {
            ActiveView::SkillBrowser => "Skill Browser",
            ActiveView::GraphView => "Graph View",
            ActiveView::InstallDashboard => "Install Dashboard",
            ActiveView::HealthPanel => "Health Panel",
        }
    }
}

/// Main TUI application state
pub struct App {
    /// Loaded configuration
    pub config: Config,
    /// Discovered skills
    pub skills: Vec<Skill>,
    /// Currently active view
    pub active_view: ActiveView,
    /// Status message to display
    pub status_message: String,
    /// Whether the app should quit
    pub should_quit: bool,
    /// Skill browser view state
    pub skill_browser_state: skill_browser::SkillBrowserState,
}

impl App {
    /// Create a new TUI app with the given config and skills
    pub fn new(config: Config, skills: Vec<Skill>) -> Self {
        let skill_browser_state = skill_browser::SkillBrowserState::new(&skills);
        App {
            config,
            skills,
            active_view: ActiveView::SkillBrowser,
            status_message: "Ready".to_string(),
            should_quit: false,
            skill_browser_state,
        }
    }

    /// Switch to the next view
    pub fn next_view(&mut self) {
        self.active_view = self.active_view.next();
        self.status_message = format!("Switched to {}", self.active_view.name());
    }

    /// Switch to the previous view
    pub fn prev_view(&mut self) {
        self.active_view = self.active_view.prev();
        self.status_message = format!("Switched to {}", self.active_view.name());
    }

    /// Set a specific view
    pub fn set_view(&mut self, view: ActiveView) {
        self.active_view = view;
        self.status_message = format!("Switched to {}", self.active_view.name());
    }

    /// Mark the app to quit
    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}

/// Run the TUI application
pub fn run(config: Config, skills: Vec<Skill>) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Setup panic hook to restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic);
    }));

    // Create app state
    let mut app = App::new(config, skills);

    // Run the event loop
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

/// Main event loop
fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    let tick_rate = Duration::from_millis(250);

    loop {
        terminal.draw(|f| ui(f, app))?;

        // Handle events with a timeout
        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                // Handle search mode for skill browser
                if app.skill_browser_state.search_active {
                    match key.code {
                        KeyCode::Char(c) => {
                            let mut filter = app.skill_browser_state.filter.clone();
                            filter.push(c);
                            app.skill_browser_state.update_filter(filter, &app.skills);
                        }
                        KeyCode::Backspace => {
                            let mut filter = app.skill_browser_state.filter.clone();
                            filter.pop();
                            app.skill_browser_state.update_filter(filter, &app.skills);
                        }
                        KeyCode::Enter | KeyCode::Esc => {
                            app.skill_browser_state.search_active = false;
                            app.status_message = "Search mode deactivated".to_string();
                        }
                        _ => {}
                    }
                } else {
                    // Normal navigation mode
                    match key.code {
                        KeyCode::Char('q') => app.quit(),
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            app.quit()
                        }
                        KeyCode::Tab => app.next_view(),
                        KeyCode::BackTab => app.prev_view(),
                        KeyCode::Char('?') => {
                            app.status_message =
                                "q: quit | Tab: next view | Shift+Tab: prev view | ?: help"
                                    .to_string()
                        }
                        KeyCode::Char('1') => app.set_view(ActiveView::SkillBrowser),
                        KeyCode::Char('2') => app.set_view(ActiveView::GraphView),
                        KeyCode::Char('3') => app.set_view(ActiveView::InstallDashboard),
                        KeyCode::Char('4') => app.set_view(ActiveView::HealthPanel),
                        // View-specific keys
                        KeyCode::Char('j') | KeyCode::Down
                            if app.active_view == ActiveView::SkillBrowser =>
                        {
                            app.skill_browser_state.next();
                        }
                        KeyCode::Char('k') | KeyCode::Up
                            if app.active_view == ActiveView::SkillBrowser =>
                        {
                            app.skill_browser_state.previous();
                        }
                        KeyCode::Char('/') if app.active_view == ActiveView::SkillBrowser => {
                            app.skill_browser_state.search_active = true;
                            app.status_message = "Search mode (Esc to exit)".to_string();
                        }
                        KeyCode::Esc if app.active_view == ActiveView::SkillBrowser => {
                            app.skill_browser_state
                                .update_filter(String::new(), &app.skills);
                            app.status_message = "Filter cleared".to_string();
                        }
                        _ => {}
                    }
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

/// Draw the UI
fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(f.area());

    // Main view area - dispatch to appropriate view renderer
    match app.active_view {
        ActiveView::SkillBrowser => {
            skill_browser::render(
                f,
                chunks[0],
                &app.config,
                &app.skills,
                &mut app.skill_browser_state,
            );
        }
        _ => {
            // Placeholder for other views
            let view_content = render_view_placeholder(app);
            let view_block = Block::default()
                .title(format!(" {} ", app.active_view.name()))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan));
            let view_widget = Paragraph::new(view_content).block(view_block);
            f.render_widget(view_widget, chunks[0]);
        }
    }

    // Status bar
    let status_spans = vec![
        Span::styled(
            format!(" {} ", app.active_view.name()),
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::raw(&app.status_message),
        Span::raw(" | "),
        Span::styled("q", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": quit | "),
        Span::styled("Tab", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": next view | "),
        Span::styled("1-4", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": jump | "),
        Span::styled("?", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": help"),
    ];
    let status_bar = Paragraph::new(Line::from(status_spans))
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));
    f.render_widget(status_bar, chunks[1]);
}

/// Render placeholder content for views that aren't implemented yet
fn render_view_placeholder(app: &App) -> String {
    let skill_count = app.skills.len();

    match app.active_view {
        ActiveView::SkillBrowser => {
            // Should never reach here - skill browser is implemented
            "Error: skill browser should be rendered by skill_browser::render".to_string()
        }
        ActiveView::GraphView => {
            format!(
                "Graph View (coming soon)\n\n\
                 {} skills in dependency graph\n\n\
                 This view will show:\n\
                 - Box-drawing dependency graph\n\
                 - Navigate between connected skills\n\
                 - Highlight clusters with color\n\
                 - Show dangling references in red",
                skill_count
            )
        }
        ActiveView::InstallDashboard => {
            let global_targets = app.config.global.targets.len();
            format!(
                "Install Dashboard (coming soon)\n\n\
                 {} global target(s)\n\n\
                 This view will show:\n\
                 - Current state of all target directories\n\
                 - One-key install, clean, reinstall\n\
                 - Diff view: what would change on next install",
                global_targets
            )
        }
        ActiveView::HealthPanel => {
            format!(
                "Health Panel (coming soon)\n\n\
                 {} skills loaded\n\n\
                 This view will show:\n\
                 - Live results from check analysis\n\
                 - Color-coded severity (errors/warnings/info)\n\
                 - Navigate directly to problem skills\n\
                 - Actionable fix suggestions",
                skill_count
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> Config {
        let toml = r#"
[sources]
skills = ["/test/skills"]

[global]
targets = ["/test/targets"]
skills = []
        "#;
        toml::from_str(toml).unwrap()
    }

    #[test]
    fn should_create_app_with_default_state() {
        // Given
        let config = test_config();
        let skills = vec![];

        // When
        let app = App::new(config, skills);

        // Then
        assert_eq!(app.active_view, ActiveView::SkillBrowser);
        assert_eq!(app.status_message, "Ready");
        assert!(!app.should_quit);
    }

    #[test]
    fn should_cycle_to_next_view() {
        // Given
        let mut app = App::new(test_config(), vec![]);

        // When
        app.next_view();

        // Then
        assert_eq!(app.active_view, ActiveView::GraphView);

        // When
        app.next_view();
        app.next_view();
        app.next_view();

        // Then (should wrap around)
        assert_eq!(app.active_view, ActiveView::SkillBrowser);
    }

    #[test]
    fn should_cycle_to_prev_view() {
        // Given
        let mut app = App::new(test_config(), vec![]);

        // When
        app.prev_view();

        // Then (should wrap around)
        assert_eq!(app.active_view, ActiveView::HealthPanel);

        // When
        app.prev_view();

        // Then
        assert_eq!(app.active_view, ActiveView::InstallDashboard);
    }

    #[test]
    fn should_set_specific_view() {
        // Given
        let mut app = App::new(test_config(), vec![]);

        // When
        app.set_view(ActiveView::HealthPanel);

        // Then
        assert_eq!(app.active_view, ActiveView::HealthPanel);
    }

    #[test]
    fn should_set_quit_flag() {
        // Given
        let mut app = App::new(test_config(), vec![]);

        // When
        app.quit();

        // Then
        assert!(app.should_quit);
    }

    #[test]
    fn should_update_status_message_on_view_change() {
        // Given
        let mut app = App::new(test_config(), vec![]);

        // When
        app.next_view();

        // Then
        assert!(app.status_message.contains("Graph View"));
    }
}
