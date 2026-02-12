//! Interactive TUI for skill management (requires `tui` feature)

#[cfg(feature = "graph")]
mod graph_view;
mod overview;
mod pipeline;
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

use crate::commands::check::{self, Finding};
use crate::config::Config;
#[cfg(feature = "graph")]
use crate::graph::SkillGraph;
use crate::skill::{self, Skill};

/// Which view is currently active
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
    SystemOverview,
    SkillBrowser,
    PipelineView,
    GraphView,
}

impl ActiveView {
    /// Get the next view in the cycle
    fn next(self) -> Self {
        match self {
            ActiveView::SystemOverview => ActiveView::SkillBrowser,
            ActiveView::SkillBrowser => ActiveView::PipelineView,
            ActiveView::PipelineView => ActiveView::GraphView,
            ActiveView::GraphView => ActiveView::SystemOverview,
        }
    }

    /// Get the previous view in the cycle
    fn prev(self) -> Self {
        match self {
            ActiveView::SystemOverview => ActiveView::GraphView,
            ActiveView::SkillBrowser => ActiveView::SystemOverview,
            ActiveView::PipelineView => ActiveView::SkillBrowser,
            ActiveView::GraphView => ActiveView::PipelineView,
        }
    }

    /// Get the display name of the view
    fn name(self) -> &'static str {
        match self {
            ActiveView::SystemOverview => "System Overview",
            ActiveView::SkillBrowser => "Skill Explorer",
            ActiveView::PipelineView => "Pipeline View",
            ActiveView::GraphView => "Graph Explorer",
        }
    }
}

/// Main TUI application state
pub struct App {
    /// Loaded configuration
    pub config: Config,
    /// Discovered skills
    pub skills: Vec<Skill>,
    /// Skill dependency graph (if graph feature enabled)
    #[cfg(feature = "graph")]
    pub graph: Option<SkillGraph>,
    /// Health check findings
    pub findings: Vec<Finding>,
    /// Currently active view
    pub active_view: ActiveView,
    /// Status message to display
    pub status_message: String,
    /// Whether the app should quit
    pub should_quit: bool,
    /// System overview state
    pub overview_state: overview::OverviewState,
    /// Skill browser view state
    pub skill_browser_state: skill_browser::SkillBrowserState,
    /// Pipeline view state
    pub pipeline_state: pipeline::PipelineState,
    /// Graph view state
    #[cfg(feature = "graph")]
    pub graph_view_state: graph_view::GraphViewState,
}

impl App {
    /// Create a new TUI app with the given config and skills
    pub fn new(config: Config, skills: Vec<Skill>) -> Self {
        // Build graph if feature enabled
        #[cfg(feature = "graph")]
        let graph = {
            let mut crossrefs = std::collections::HashMap::new();
            let skill_names: std::collections::HashSet<String> =
                skills.iter().map(|s| s.name.clone()).collect();
            for skill in &skills {
                if let Ok(content) = std::fs::read_to_string(&skill.skill_file) {
                    let refs = skill::extract_references_with_filter(
                        &content,
                        &skill.name,
                        Some(&skill_names),
                    );
                    if !refs.is_empty() {
                        crossrefs.insert(skill.name.clone(), refs);
                    }
                }
            }
            Some(SkillGraph::from_skills(&crossrefs, &skills))
        };

        // Run health checks
        let findings = check::check(&config, None, false).unwrap_or_default();

        let mut overview_state = overview::OverviewState::new();
        overview_state.refresh(&config, &skills);
        let skill_browser_state = skill_browser::SkillBrowserState::new(&skills);
        let mut pipeline_state = pipeline::PipelineState::new();
        pipeline_state.refresh(&config, &skills);
        #[cfg(feature = "graph")]
        let mut graph_view_state = graph_view::GraphViewState::new();
        #[cfg(feature = "graph")]
        graph_view_state.refresh(&config, &skills);
        App {
            config,
            skills,
            #[cfg(feature = "graph")]
            graph,
            findings,
            active_view: ActiveView::SystemOverview,
            status_message: "Ready".to_string(),
            should_quit: false,
            overview_state,
            skill_browser_state,
            pipeline_state,
            #[cfg(feature = "graph")]
            graph_view_state,
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
                        KeyCode::Char('1') => app.set_view(ActiveView::SystemOverview),
                        KeyCode::Char('2') => app.set_view(ActiveView::SkillBrowser),
                        KeyCode::Char('3') => app.set_view(ActiveView::PipelineView),
                        KeyCode::Char('4') => app.set_view(ActiveView::GraphView),
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
                        KeyCode::Enter if app.active_view == ActiveView::SkillBrowser => {
                            app.skill_browser_state.toggle_mode();
                            let mode_name = match app.skill_browser_state.mode {
                                skill_browser::ExplorerMode::List => "List mode",
                                skill_browser::ExplorerMode::Profile => "Profile mode",
                            };
                            app.status_message = format!("Switched to {}", mode_name);
                        }
                        KeyCode::Esc if app.active_view == ActiveView::SkillBrowser => {
                            // If in profile mode, return to list mode
                            // Otherwise, clear filter
                            if app.skill_browser_state.mode == skill_browser::ExplorerMode::Profile
                            {
                                app.skill_browser_state.mode = skill_browser::ExplorerMode::List;
                                app.status_message = "Returned to list mode".to_string();
                            } else {
                                app.skill_browser_state
                                    .update_filter(String::new(), &app.skills);
                                app.status_message = "Filter cleared".to_string();
                            }
                        }
                        // System overview refresh
                        KeyCode::Char('r') if app.active_view == ActiveView::SystemOverview => {
                            app.overview_state.refresh(&app.config, &app.skills);
                            app.status_message = "Overview refreshed".to_string();
                        }
                        // Pipeline view navigation
                        KeyCode::Char('j') | KeyCode::Down
                            if app.active_view == ActiveView::PipelineView =>
                        {
                            app.pipeline_state.next();
                        }
                        KeyCode::Char('k') | KeyCode::Up
                            if app.active_view == ActiveView::PipelineView =>
                        {
                            app.pipeline_state.previous();
                        }
                        KeyCode::Char('r') if app.active_view == ActiveView::PipelineView => {
                            app.pipeline_state.refresh(&app.config, &app.skills);
                            app.status_message = "Pipelines refreshed".to_string();
                        }
                        // Graph view navigation
                        #[cfg(feature = "graph")]
                        KeyCode::Char('j') | KeyCode::Down
                            if app.active_view == ActiveView::GraphView =>
                        {
                            app.graph_view_state.next();
                        }
                        #[cfg(feature = "graph")]
                        KeyCode::Char('k') | KeyCode::Up
                            if app.active_view == ActiveView::GraphView =>
                        {
                            app.graph_view_state.previous();
                        }
                        #[cfg(feature = "graph")]
                        KeyCode::Char('r') if app.active_view == ActiveView::GraphView => {
                            app.graph_view_state.refresh(&app.config, &app.skills);
                            app.status_message = "Graph refreshed".to_string();
                        }
                        #[cfg(feature = "graph")]
                        KeyCode::Enter if app.active_view == ActiveView::GraphView => {
                            if app.graph_view_state.mode == graph_view::NavigationMode::Browse {
                                app.graph_view_state.toggle_mode();
                                app.status_message =
                                    "Focus mode - follow edges with Enter".to_string();
                            } else {
                                app.graph_view_state.follow_edge();
                                app.status_message = "Following edge...".to_string();
                            }
                        }
                        #[cfg(feature = "graph")]
                        KeyCode::Backspace if app.active_view == ActiveView::GraphView => {
                            app.graph_view_state.navigate_back();
                            app.status_message = "Navigated back".to_string();
                        }
                        #[cfg(feature = "graph")]
                        KeyCode::Esc if app.active_view == ActiveView::GraphView => {
                            if app.graph_view_state.mode == graph_view::NavigationMode::Focus {
                                app.graph_view_state.mode = graph_view::NavigationMode::Browse;
                                app.graph_view_state.trail.clear();
                                app.status_message = "Returned to browse mode".to_string();
                            }
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
        ActiveView::SystemOverview => {
            overview::render(f, chunks[0], &app.overview_state);
        }
        ActiveView::SkillBrowser => {
            skill_browser::render(
                f,
                chunks[0],
                &app.config,
                &app.skills,
                #[cfg(feature = "graph")]
                app.graph.as_ref(),
                #[cfg(not(feature = "graph"))]
                None,
                &app.findings,
                &mut app.skill_browser_state,
            );
        }
        ActiveView::PipelineView => {
            pipeline::render(
                f,
                chunks[0],
                &app.config,
                &app.skills,
                &mut app.pipeline_state,
            );
        }
        #[cfg(feature = "graph")]
        ActiveView::GraphView => {
            graph_view::render(f, chunks[0], &mut app.graph_view_state);
        }
        #[cfg(not(feature = "graph"))]
        ActiveView::GraphView => {
            let view_content =
                "Graph view requires the 'graph' feature.\n\nRebuild with --features graph,tui"
                    .to_string();
            let view_block = Block::default()
                .title(" Graph View ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red));
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
        assert_eq!(app.active_view, ActiveView::SystemOverview);
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
        assert_eq!(app.active_view, ActiveView::SkillBrowser);

        // When
        app.next_view();
        app.next_view();
        app.next_view();

        // Then (should wrap around)
        assert_eq!(app.active_view, ActiveView::SystemOverview);
    }

    #[test]
    fn should_cycle_to_prev_view() {
        // Given
        let mut app = App::new(test_config(), vec![]);

        // When
        app.prev_view();

        // Then (should wrap around)
        assert_eq!(app.active_view, ActiveView::GraphView);

        // When
        app.prev_view();

        // Then
        assert_eq!(app.active_view, ActiveView::PipelineView);
    }

    #[test]
    fn should_set_specific_view() {
        // Given
        let mut app = App::new(test_config(), vec![]);

        // When
        app.set_view(ActiveView::SkillBrowser);

        // Then
        assert_eq!(app.active_view, ActiveView::SkillBrowser);
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
        assert!(app.status_message.contains("Skill Explorer"));
    }
}
