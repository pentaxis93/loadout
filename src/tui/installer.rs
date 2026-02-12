//! Install dashboard view - manage installations

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};
use std::path::PathBuf;

use crate::config::Config;
use crate::linker;
use crate::skill::Skill;

/// State for the install dashboard view
pub struct InstallerState {
    /// Last operation result message
    pub last_operation: Option<String>,
    /// Whether an operation is in progress
    pub operation_in_progress: bool,
}

impl InstallerState {
    /// Create a new installer state
    pub fn new() -> Self {
        InstallerState {
            last_operation: None,
            operation_in_progress: false,
        }
    }

    /// Perform install operation
    pub fn install(&mut self, config: &Config, skills: &[Skill]) -> Result<String, String> {
        if self.operation_in_progress {
            return Err("Operation already in progress".to_string());
        }

        self.operation_in_progress = true;
        let result = perform_install(config, skills);
        self.operation_in_progress = false;

        match result {
            Ok(msg) => {
                self.last_operation = Some(format!("✓ {}", msg));
                Ok(msg)
            }
            Err(e) => {
                self.last_operation = Some(format!("✗ {}", e));
                Err(e)
            }
        }
    }

    /// Perform clean operation
    pub fn clean(&mut self, config: &Config) -> Result<String, String> {
        if self.operation_in_progress {
            return Err("Operation already in progress".to_string());
        }

        self.operation_in_progress = true;
        let result = perform_clean(config);
        self.operation_in_progress = false;

        match result {
            Ok(msg) => {
                self.last_operation = Some(format!("✓ {}", msg));
                Ok(msg)
            }
            Err(e) => {
                self.last_operation = Some(format!("✗ {}", e));
                Err(e)
            }
        }
    }
}

/// Render the install dashboard view
pub fn render(
    f: &mut Frame,
    area: Rect,
    config: &Config,
    skills: &[Skill],
    state: &InstallerState,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Target list
            Constraint::Length(5), // Operations
        ])
        .split(area);

    render_header(f, chunks[0], config, skills, state);
    render_targets(f, chunks[1], config);
    render_operations(f, chunks[2], state);
}

/// Render the header with summary info
fn render_header(
    f: &mut Frame,
    area: Rect,
    config: &Config,
    skills: &[Skill],
    state: &InstallerState,
) {
    let active_skills = get_active_skills(config, skills);
    let targets = &config.global.targets;

    let text = if state.operation_in_progress {
        "Operation in progress...".to_string()
    } else {
        format!(
            "{} active skills → {} target(s)",
            active_skills.len(),
            targets.len()
        )
    };

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .title(" Install Dashboard ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .style(Style::default().add_modifier(Modifier::BOLD));

    f.render_widget(paragraph, area);
}

/// Render the target directories list
fn render_targets(f: &mut Frame, area: Rect, config: &Config) {
    let targets = &config.global.targets;

    let items: Vec<ListItem> = targets
        .iter()
        .map(|target| {
            let state = check_target_state(target);
            let (icon, color, status_text) = match state {
                TargetState::Managed(count) => {
                    ("●", Color::Green, format!("{} managed symlinks", count))
                }
                TargetState::Unmanaged => ("○", Color::Gray, "not managed".to_string()),
                TargetState::Missing => ("✗", Color::Red, "directory missing".to_string()),
            };

            let line = Line::from(vec![
                Span::styled(icon, Style::default().fg(color)),
                Span::raw(" "),
                Span::raw(target.display().to_string()),
                Span::raw(" "),
                Span::styled(
                    format!("({})", status_text),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" Target Directories ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    f.render_widget(list, area);
}

/// Render the operations panel
fn render_operations(f: &mut Frame, area: Rect, state: &InstallerState) {
    let mut lines = vec![Line::from(vec![
        Span::styled("i", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": install | "),
        Span::styled("c", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(": clean"),
    ])];

    if let Some(last_op) = &state.last_operation {
        lines.push(Line::from(""));
        lines.push(Line::from(last_op.clone()));
    }

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Operations ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

/// Target directory state
#[derive(Debug, PartialEq, Eq)]
enum TargetState {
    Managed(usize), // Number of managed symlinks
    Unmanaged,
    Missing,
}

fn check_target_state(target: &PathBuf) -> TargetState {
    if !target.exists() {
        return TargetState::Missing;
    }

    if !linker::is_managed(target) {
        return TargetState::Unmanaged;
    }

    // Count managed symlinks
    let count = std::fs::read_dir(target)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|e| e.path().is_symlink())
                .count()
        })
        .unwrap_or(0);

    TargetState::Managed(count)
}

/// Get list of skills that should be installed (global + projects)
fn get_active_skills<'a>(config: &Config, skills: &'a [Skill]) -> Vec<&'a Skill> {
    let mut active_names: Vec<String> = config.global.skills.clone();

    // Add project skills (simplified - we don't have project path context in TUI)
    for project in config.projects.values() {
        active_names.extend(project.skills.clone());
    }

    // Deduplicate
    active_names.sort();
    active_names.dedup();

    // Find the actual skill structs
    active_names
        .iter()
        .filter_map(|name| skills.iter().find(|s| &s.name == name))
        .collect()
}

/// Perform the install operation
fn perform_install(config: &Config, skills: &[Skill]) -> Result<String, String> {
    let active_skills = get_active_skills(config, skills);
    let targets = &config.global.targets;

    for target in targets {
        // Create target directory if it doesn't exist
        if !target.exists() {
            std::fs::create_dir_all(target)
                .map_err(|e| format!("Failed to create target directory: {}", e))?;
        }

        for skill in &active_skills {
            linker::link_skill(&skill.name, &skill.skill_file, target)
                .map_err(|e| format!("Failed to link {}: {}", skill.name, e))?;
        }
    }

    Ok(format!(
        "Installed {} skills to {} target(s)",
        active_skills.len(),
        targets.len()
    ))
}

/// Perform the clean operation
fn perform_clean(config: &Config) -> Result<String, String> {
    let targets = &config.global.targets;
    let mut total_removed = 0;

    for target in targets {
        if !target.exists() {
            continue;
        }

        let removed = linker::clean_target(target)
            .map_err(|e| format!("Failed to clean {}: {}", target.display(), e))?;
        total_removed += removed.len();
    }

    Ok(format!(
        "Removed {} symlinks from {} target(s)",
        total_removed,
        targets.len()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_initialize_with_no_operation() {
        // Given / When
        let state = InstallerState::new();

        // Then
        assert_eq!(state.last_operation, None);
        assert!(!state.operation_in_progress);
    }

    #[test]
    fn should_prevent_concurrent_operations() {
        // Given
        let mut state = InstallerState::new();
        state.operation_in_progress = true;

        // When
        let result = state.install(&test_config(), &[]);

        // Then
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Operation already in progress");
    }

    #[test]
    fn should_get_active_skills_from_global() {
        // Given
        let config = test_config();
        let skills = vec![
            test_skill("test-skill"), // This matches config.global.skills
            test_skill("skill-b"),
            test_skill("skill-c"),
        ];

        // When
        let active = get_active_skills(&config, &skills);

        // Then
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].name, "test-skill");
    }

    #[test]
    fn should_deduplicate_active_skills() {
        // Given
        let mut config = test_config();
        // Add the same skill to both global and a project
        config.projects.insert(
            PathBuf::from("/project"),
            crate::config::Project {
                skills: vec!["test-skill".to_string()],
                inherit: true,
            },
        );
        let skills = vec![test_skill("test-skill")];

        // When
        let active = get_active_skills(&config, &skills);

        // Then (should only appear once)
        assert_eq!(active.len(), 1);
    }

    fn test_config() -> Config {
        let toml = r#"
[sources]
skills = ["/test/skills"]

[global]
targets = ["/test/targets"]
skills = ["test-skill"]
        "#;
        toml::from_str(toml).unwrap()
    }

    fn test_skill(name: &str) -> Skill {
        use crate::skill::frontmatter::Frontmatter;

        Skill {
            name: name.to_string(),
            path: PathBuf::from(format!("/test/{}", name)),
            skill_file: PathBuf::from(format!("/test/{}/SKILL.md", name)),
            frontmatter: Frontmatter {
                name: name.to_string(),
                description: format!("Test skill {}", name),
                tags: None,
                disable_model_invocation: None,
                user_invocable: None,
                allowed_tools: None,
                context: None,
                agent: None,
                model: None,
                argument_hint: None,
                license: None,
                compatibility: None,
                metadata: None,
                pipeline: None,
            },
        }
    }
}
