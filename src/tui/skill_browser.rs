//! Skill browser view - list + detail pane

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::config::Config;
use crate::skill::Skill;

/// State for the skill browser view
pub struct SkillBrowserState {
    /// List selection state
    pub list_state: ListState,
    /// Current search filter
    pub filter: String,
    /// Whether search mode is active
    pub search_active: bool,
    /// Cached filtered skills (indices into the full skill list)
    filtered_indices: Vec<usize>,
}

impl SkillBrowserState {
    /// Create a new skill browser state
    pub fn new(skills: &[Skill]) -> Self {
        let mut state = SkillBrowserState {
            list_state: ListState::default(),
            filter: String::new(),
            search_active: false,
            filtered_indices: (0..skills.len()).collect(),
        };
        if !skills.is_empty() {
            state.list_state.select(Some(0));
        }
        state
    }

    /// Update the filter and recompute filtered indices
    pub fn update_filter(&mut self, filter: String, skills: &[Skill]) {
        self.filter = filter;
        self.recompute_filter(skills);
    }

    /// Recompute which skills match the current filter
    fn recompute_filter(&mut self, skills: &[Skill]) {
        if self.filter.is_empty() {
            self.filtered_indices = (0..skills.len()).collect();
        } else {
            let filter_lower = self.filter.to_lowercase();
            self.filtered_indices = skills
                .iter()
                .enumerate()
                .filter(|(_, s)| Self::matches_filter(s, &filter_lower))
                .map(|(i, _)| i)
                .collect();
        }

        // Reset selection to first item
        if !self.filtered_indices.is_empty() {
            self.list_state.select(Some(0));
        } else {
            self.list_state.select(None);
        }
    }

    /// Check if a skill matches the filter
    fn matches_filter(skill: &Skill, filter_lower: &str) -> bool {
        // Match on name
        if skill.name.to_lowercase().contains(filter_lower) {
            return true;
        }

        // Match on description
        if skill
            .frontmatter
            .description
            .to_lowercase()
            .contains(filter_lower)
        {
            return true;
        }

        // Match on tags
        if let Some(tags) = &skill.frontmatter.tags {
            if tags.iter().any(|t| t.to_lowercase().contains(filter_lower)) {
                return true;
            }
        }

        false
    }

    /// Get the currently selected skill (if any)
    pub fn selected_skill<'a>(&self, skills: &'a [Skill]) -> Option<&'a Skill> {
        let selected_filtered_idx = self.list_state.selected()?;
        let skill_idx = *self.filtered_indices.get(selected_filtered_idx)?;
        skills.get(skill_idx)
    }

    /// Move selection down
    pub fn next(&mut self) {
        if self.filtered_indices.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.filtered_indices.len() - 1 {
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
        if self.filtered_indices.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.filtered_indices.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    /// Get the filtered skill list
    pub fn filtered_skills<'a>(&self, skills: &'a [Skill]) -> Vec<&'a Skill> {
        self.filtered_indices
            .iter()
            .filter_map(|&i| skills.get(i))
            .collect()
    }
}

/// Render the skill browser view
pub fn render(
    f: &mut Frame,
    area: Rect,
    config: &Config,
    skills: &[Skill],
    state: &mut SkillBrowserState,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    render_skill_list(f, chunks[0], config, skills, state);
    render_detail_pane(f, chunks[1], skills, state);
}

/// Render the skill list (left pane)
fn render_skill_list(
    f: &mut Frame,
    area: Rect,
    config: &Config,
    skills: &[Skill],
    state: &mut SkillBrowserState,
) {
    let filtered_skills = state.filtered_skills(skills);

    // Build list items with status indicators
    let items: Vec<ListItem> = filtered_skills
        .iter()
        .map(|s| {
            let status = determine_status(s, config);
            let (icon, color) = status_icon_and_color(&status);

            let line = Line::from(vec![
                Span::styled(icon, Style::default().fg(color)),
                Span::raw(" "),
                Span::raw(&s.name),
            ]);

            ListItem::new(line)
        })
        .collect();

    let title = if state.search_active {
        format!(
            " Skills (filtering: {}) ",
            if state.filter.is_empty() {
                "...".to_string()
            } else {
                state.filter.clone()
            }
        )
    } else if !state.filter.is_empty() {
        format!(" Skills (filter: {}) ", state.filter)
    } else {
        " Skills ".to_string()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, area, &mut state.list_state);
}

/// Render the detail pane (right pane)
fn render_detail_pane(f: &mut Frame, area: Rect, skills: &[Skill], state: &SkillBrowserState) {
    let content = if let Some(skill) = state.selected_skill(skills) {
        format_skill_detail(skill)
    } else if state.filtered_indices.is_empty() {
        "No skills match the current filter.\n\nPress Esc to clear filter.".to_string()
    } else {
        "No skills discovered.\n\nCheck your config sources.".to_string()
    };

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .title(" Detail ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

/// Determine a skill's status
#[derive(Debug, PartialEq, Eq)]
enum SkillStatus {
    Active, // In global.skills or a project
    Available, // In sources but not activated
            // Note: "installed" (symlink exists) and "orphaned" require filesystem checks
            // which we'll skip for now to keep it simple. Could add later.
}

fn determine_status(skill: &Skill, config: &Config) -> SkillStatus {
    // Check if skill is in global.skills
    if config.global.skills.contains(&skill.name) {
        return SkillStatus::Active;
    }

    // Check if skill is in any project
    for project in config.projects.values() {
        if project.skills.contains(&skill.name) {
            return SkillStatus::Active;
        }
    }

    SkillStatus::Available
}

fn status_icon_and_color(status: &SkillStatus) -> (&'static str, Color) {
    match status {
        SkillStatus::Active => ("●", Color::Green),
        SkillStatus::Available => ("○", Color::Gray),
    }
}

/// Format skill detail for display
fn format_skill_detail(skill: &Skill) -> String {
    let mut lines = Vec::new();

    // Name and description
    lines.push(format!("Name: {}", skill.name));
    lines.push(String::new());
    lines.push("Description:".to_string());
    lines.push(skill.frontmatter.description.clone());
    lines.push(String::new());

    // Path
    lines.push(format!("Path: {}", skill.path.display()));
    lines.push(String::new());

    // Tags
    if let Some(tags) = &skill.frontmatter.tags {
        lines.push(format!("Tags: {}", tags.join(", ")));
        lines.push(String::new());
    }

    // Pipelines
    if let Some(pipeline) = &skill.frontmatter.pipeline {
        lines.push("Pipelines:".to_string());
        for (name, stage) in pipeline {
            let mut deps = Vec::new();
            if let Some(after) = &stage.after {
                deps.push(format!("after: {}", after.join(", ")));
            }
            if let Some(before) = &stage.before {
                deps.push(format!("before: {}", before.join(", ")));
            }
            let dep_str = if deps.is_empty() {
                String::new()
            } else {
                format!(" ({})", deps.join("; "))
            };
            lines.push(format!(
                "  - {}: stage={}, order={}{}",
                name, stage.stage, stage.order, dep_str
            ));
        }
        lines.push(String::new());
    }

    // Optional Claude Code fields
    if let Some(val) = &skill.frontmatter.user_invocable {
        lines.push(format!("User invocable: {}", val));
    }
    if let Some(val) = &skill.frontmatter.disable_model_invocation {
        lines.push(format!("Disable model invocation: {}", val));
    }
    if let Some(val) = &skill.frontmatter.allowed_tools {
        lines.push(format!("Allowed tools: {}", val));
    }
    if let Some(val) = &skill.frontmatter.context {
        lines.push(format!("Context: {}", val));
    }
    if let Some(val) = &skill.frontmatter.agent {
        lines.push(format!("Agent: {}", val));
    }
    if let Some(val) = &skill.frontmatter.model {
        lines.push(format!("Model: {}", val));
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::frontmatter::Frontmatter;
    use std::path::PathBuf;

    fn test_skill(name: &str, description: &str, tags: Option<Vec<String>>) -> Skill {
        Skill {
            name: name.to_string(),
            path: PathBuf::from(format!("/test/{}", name)),
            skill_file: PathBuf::from(format!("/test/{}/SKILL.md", name)),
            frontmatter: Frontmatter {
                name: name.to_string(),
                description: description.to_string(),
                tags,
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

    #[test]
    fn should_initialize_with_first_skill_selected() {
        // Given
        let skills = vec![test_skill("skill-a", "Test A", None)];

        // When
        let state = SkillBrowserState::new(&skills);

        // Then
        assert_eq!(state.list_state.selected(), Some(0));
        assert_eq!(state.filtered_indices, vec![0]);
    }

    #[test]
    fn should_filter_by_name() {
        // Given
        let skills = vec![
            test_skill("blog-edit", "Edit blog posts", None),
            test_skill("tech-blog", "Tech articles", None),
            test_skill("skill-craft", "Create skills", None),
        ];
        let mut state = SkillBrowserState::new(&skills);

        // When
        state.update_filter("blog".to_string(), &skills);

        // Then
        assert_eq!(state.filtered_indices, vec![0, 1]);
    }

    #[test]
    fn should_filter_by_description() {
        // Given
        let skills = vec![
            test_skill("skill-a", "Create and edit skills", None),
            test_skill("skill-b", "Manage blog content", None),
            test_skill("skill-c", "Review code", None),
        ];
        let mut state = SkillBrowserState::new(&skills);

        // When
        state.update_filter("edit".to_string(), &skills);

        // Then
        assert_eq!(state.filtered_indices, vec![0]);
    }

    #[test]
    fn should_filter_by_tag() {
        // Given
        let skills = vec![
            test_skill("skill-a", "Test A", Some(vec!["blog".to_string()])),
            test_skill("skill-b", "Test B", Some(vec!["writing".to_string()])),
            test_skill(
                "skill-c",
                "Test C",
                Some(vec!["blog".to_string(), "meta".to_string()]),
            ),
        ];
        let mut state = SkillBrowserState::new(&skills);

        // When
        state.update_filter("blog".to_string(), &skills);

        // Then
        assert_eq!(state.filtered_indices, vec![0, 2]);
    }

    #[test]
    fn should_reset_selection_after_filter() {
        // Given
        let skills = vec![
            test_skill("skill-a", "Test A", None),
            test_skill("skill-b", "Test B", None),
        ];
        let mut state = SkillBrowserState::new(&skills);
        state.list_state.select(Some(1)); // Select second item

        // When
        state.update_filter("a".to_string(), &skills);

        // Then (filter matches only first skill, selection should reset)
        assert_eq!(state.list_state.selected(), Some(0));
        assert_eq!(state.filtered_indices, vec![0]);
    }

    #[test]
    fn should_handle_empty_filter_result() {
        // Given
        let skills = vec![test_skill("skill-a", "Test A", None)];
        let mut state = SkillBrowserState::new(&skills);

        // When
        state.update_filter("nonexistent".to_string(), &skills);

        // Then
        assert_eq!(state.list_state.selected(), None);
        assert_eq!(state.filtered_indices, Vec::<usize>::new());
    }

    #[test]
    fn should_clear_filter_when_filter_is_empty() {
        // Given
        let skills = vec![
            test_skill("skill-a", "Test A", None),
            test_skill("skill-b", "Test B", None),
        ];
        let mut state = SkillBrowserState::new(&skills);
        state.update_filter("a".to_string(), &skills);

        // When
        state.update_filter(String::new(), &skills);

        // Then
        assert_eq!(state.filtered_indices, vec![0, 1]);
    }

    #[test]
    fn should_move_selection_down() {
        // Given
        let skills = vec![
            test_skill("skill-a", "Test A", None),
            test_skill("skill-b", "Test B", None),
            test_skill("skill-c", "Test C", None),
        ];
        let mut state = SkillBrowserState::new(&skills);

        // When
        state.next();

        // Then
        assert_eq!(state.list_state.selected(), Some(1));

        // When (move again)
        state.next();

        // Then
        assert_eq!(state.list_state.selected(), Some(2));
    }

    #[test]
    fn should_wrap_selection_at_end() {
        // Given
        let skills = vec![
            test_skill("skill-a", "Test A", None),
            test_skill("skill-b", "Test B", None),
        ];
        let mut state = SkillBrowserState::new(&skills);
        state.list_state.select(Some(1)); // Last item

        // When
        state.next();

        // Then (should wrap to first)
        assert_eq!(state.list_state.selected(), Some(0));
    }

    #[test]
    fn should_move_selection_up() {
        // Given
        let skills = vec![
            test_skill("skill-a", "Test A", None),
            test_skill("skill-b", "Test B", None),
        ];
        let mut state = SkillBrowserState::new(&skills);
        state.list_state.select(Some(1));

        // When
        state.previous();

        // Then
        assert_eq!(state.list_state.selected(), Some(0));
    }

    #[test]
    fn should_wrap_selection_at_start() {
        // Given
        let skills = vec![
            test_skill("skill-a", "Test A", None),
            test_skill("skill-b", "Test B", None),
        ];
        let mut state = SkillBrowserState::new(&skills);

        // When
        state.previous();

        // Then (should wrap to last)
        assert_eq!(state.list_state.selected(), Some(1));
    }
}
