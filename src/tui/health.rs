//! Health panel view - check analysis results

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::commands::check::{check, Finding, Severity};
use crate::config::Config;
use crate::skill::Skill;

/// State for the health panel view
pub struct HealthPanelState {
    /// List selection state
    pub list_state: ListState,
    /// Cached findings (recomputed when entering the view)
    findings: Vec<Finding>,
}

impl HealthPanelState {
    /// Create a new health panel state
    pub fn new() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        HealthPanelState {
            list_state: state,
            findings: Vec::new(),
        }
    }

    /// Refresh findings by running the check analysis
    pub fn refresh(&mut self, config: &Config, _skills: &[Skill]) {
        // Run check with no filter (all severities), not verbose
        self.findings = check(config, None, false).unwrap_or_default();

        // Reset selection
        if !self.findings.is_empty() {
            self.list_state.select(Some(0));
        } else {
            self.list_state.select(None);
        }
    }

    /// Get the currently selected finding
    pub fn selected_finding(&self) -> Option<&Finding> {
        let idx = self.list_state.selected()?;
        self.findings.get(idx)
    }

    /// Get the skill name from a selected finding (if it has a path)
    pub fn selected_skill_name(&self) -> Option<String> {
        let finding = self.selected_finding()?;
        finding
            .path
            .as_ref()?
            .file_name()?
            .to_str()
            .map(String::from)
    }

    /// Move selection down
    pub fn next(&mut self) {
        if self.findings.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.findings.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    /// Move selection up
    pub fn previous(&mut self) {
        if self.findings.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.findings.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    /// Get counts by severity
    pub fn counts(&self) -> (usize, usize, usize) {
        let errors = self
            .findings
            .iter()
            .filter(|f| f.severity == Severity::Error)
            .count();
        let warnings = self
            .findings
            .iter()
            .filter(|f| f.severity == Severity::Warning)
            .count();
        let info = self
            .findings
            .iter()
            .filter(|f| f.severity == Severity::Info)
            .count();
        (errors, warnings, info)
    }
}

/// Render the health panel view
pub fn render(f: &mut Frame, area: Rect, state: &mut HealthPanelState) {
    if state.findings.is_empty() {
        render_empty_state(f, area);
    } else {
        render_findings_list(f, area, state);
    }
}

/// Render the empty state (no findings)
fn render_empty_state(f: &mut Frame, area: Rect) {
    let content =
        "✓ All clear\n\nNo errors, warnings, or info findings.\n\nYour skill system is healthy!";
    let paragraph = Paragraph::new(content).block(
        Block::default()
            .title(" Health ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green)),
    );
    f.render_widget(paragraph, area);
}

/// Render the findings list
fn render_findings_list(f: &mut Frame, area: Rect, state: &mut HealthPanelState) {
    let (errors, warnings, info) = state.counts();

    // Build list items grouped by severity
    let mut items: Vec<ListItem> = Vec::new();

    // Errors first
    for finding in state
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
    {
        items.push(format_finding(finding, Severity::Error));
    }

    // Then warnings
    for finding in state
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Warning)
    {
        items.push(format_finding(finding, Severity::Warning));
    }

    // Then info
    for finding in state
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Info)
    {
        items.push(format_finding(finding, Severity::Info));
    }

    let title = format!(
        " Health ({} errors, {} warnings, {} info) ",
        errors, warnings, info
    );

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(if errors > 0 {
                    Color::Red
                } else if warnings > 0 {
                    Color::Yellow
                } else {
                    Color::Cyan
                })),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, area, &mut state.list_state);
}

/// Format a finding as a list item
fn format_finding(finding: &Finding, severity: Severity) -> ListItem<'static> {
    let (icon, color) = severity_icon_and_color(severity);

    let mut spans = vec![
        Span::styled(
            icon,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::raw(finding.message.clone()),
    ];

    if let Some(path) = &finding.path {
        spans.push(Span::raw(" ("));
        spans.push(Span::styled(
            path.display().to_string(),
            Style::default().fg(Color::Gray),
        ));
        spans.push(Span::raw(")"));
    }

    spans.push(Span::raw("\n  "));
    spans.push(Span::styled("→ ", Style::default().fg(Color::DarkGray)));
    spans.push(Span::raw(finding.fix.clone()));

    ListItem::new(vec![Line::from(spans)])
}

fn severity_icon_and_color(severity: Severity) -> (&'static str, Color) {
    match severity {
        Severity::Error => ("✗", Color::Red),
        Severity::Warning => ("⚠", Color::Yellow),
        Severity::Info => ("ℹ", Color::Cyan),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_finding(severity: Severity, message: &str, path: Option<PathBuf>) -> Finding {
        Finding {
            severity,
            message: message.to_string(),
            fix: "Test fix suggestion".to_string(),
            path,
            suppress_key: "test:key".to_string(),
        }
    }

    #[test]
    fn should_initialize_with_no_findings() {
        // Given / When
        let state = HealthPanelState::new();

        // Then
        assert_eq!(state.findings.len(), 0);
    }

    #[test]
    fn should_count_findings_by_severity() {
        // Given
        let mut state = HealthPanelState::new();
        state.findings = vec![
            test_finding(Severity::Error, "Error 1", None),
            test_finding(Severity::Error, "Error 2", None),
            test_finding(Severity::Warning, "Warning 1", None),
            test_finding(Severity::Info, "Info 1", None),
        ];

        // When
        let (errors, warnings, info) = state.counts();

        // Then
        assert_eq!(errors, 2);
        assert_eq!(warnings, 1);
        assert_eq!(info, 1);
    }

    #[test]
    fn should_move_selection_down() {
        // Given
        let mut state = HealthPanelState::new();
        state.findings = vec![
            test_finding(Severity::Error, "Finding 1", None),
            test_finding(Severity::Warning, "Finding 2", None),
        ];
        state.list_state.select(Some(0));

        // When
        state.next();

        // Then
        assert_eq!(state.list_state.selected(), Some(1));
    }

    #[test]
    fn should_wrap_selection_at_end() {
        // Given
        let mut state = HealthPanelState::new();
        state.findings = vec![
            test_finding(Severity::Error, "Finding 1", None),
            test_finding(Severity::Warning, "Finding 2", None),
        ];
        state.list_state.select(Some(1)); // Last item

        // When
        state.next();

        // Then (should wrap to first)
        assert_eq!(state.list_state.selected(), Some(0));
    }

    #[test]
    fn should_move_selection_up() {
        // Given
        let mut state = HealthPanelState::new();
        state.findings = vec![
            test_finding(Severity::Error, "Finding 1", None),
            test_finding(Severity::Warning, "Finding 2", None),
        ];
        state.list_state.select(Some(1));

        // When
        state.previous();

        // Then
        assert_eq!(state.list_state.selected(), Some(0));
    }

    #[test]
    fn should_wrap_selection_at_start() {
        // Given
        let mut state = HealthPanelState::new();
        state.findings = vec![
            test_finding(Severity::Error, "Finding 1", None),
            test_finding(Severity::Warning, "Finding 2", None),
        ];
        state.list_state.select(Some(0));

        // When
        state.previous();

        // Then (should wrap to last)
        assert_eq!(state.list_state.selected(), Some(1));
    }

    #[test]
    fn should_extract_skill_name_from_path() {
        // Given
        let mut state = HealthPanelState::new();
        state.findings = vec![test_finding(
            Severity::Error,
            "Test error",
            Some(PathBuf::from("/test/skill-name")),
        )];
        state.list_state.select(Some(0));

        // When
        let skill_name = state.selected_skill_name();

        // Then
        assert_eq!(skill_name, Some("skill-name".to_string()));
    }

    #[test]
    fn should_return_none_for_finding_without_path() {
        // Given
        let mut state = HealthPanelState::new();
        state.findings = vec![test_finding(Severity::Error, "Test error", None)];
        state.list_state.select(Some(0));

        // When
        let skill_name = state.selected_skill_name();

        // Then
        assert_eq!(skill_name, None);
    }
}
