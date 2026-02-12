//! Pipeline view - workflow visualization and navigation

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use std::collections::{HashMap, HashSet};

use crate::config::Config;
use crate::skill::Skill;

/// State for the pipeline view
pub struct PipelineState {
    /// List selection state (selects which pipeline to view)
    pub list_state: ListState,
    /// Detected pipelines
    pipelines: Vec<PipelineInfo>,
}

/// Information about a detected pipeline
pub(crate) struct PipelineInfo {
    name: String,
    stages: Vec<StageInfo>,
    skills: Vec<String>,
    gaps: Vec<String>,           // Missing skills referenced in after/before
    cross_pipeline: Vec<String>, // Skills that appear in multiple pipelines
}

pub(crate) struct StageInfo {
    name: String,
    skills: Vec<String>,
}

impl PipelineState {
    /// Create a new pipeline state
    pub fn new() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        PipelineState {
            list_state: state,
            pipelines: Vec::new(),
        }
    }

    /// Refresh pipeline data from skills
    pub fn refresh(&mut self, _config: &Config, skills: &[Skill]) {
        self.pipelines = extract_pipelines(skills);
        if !self.pipelines.is_empty() {
            self.list_state.select(Some(0));
        } else {
            self.list_state.select(None);
        }
    }

    /// Get the currently selected pipeline
    pub(crate) fn selected_pipeline(&self) -> Option<&PipelineInfo> {
        let idx = self.list_state.selected()?;
        self.pipelines.get(idx)
    }

    /// Move selection down
    pub fn next(&mut self) {
        if self.pipelines.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i >= self.pipelines.len() - 1 {
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
        if self.pipelines.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.pipelines.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }
}

/// Extract pipeline information from skills
fn extract_pipelines(skills: &[Skill]) -> Vec<PipelineInfo> {
    // Group skills by pipeline name
    let mut pipeline_map: HashMap<String, Vec<&Skill>> = HashMap::new();

    for skill in skills {
        if let Some(pipeline_def) = &skill.frontmatter.pipeline {
            for pipeline_name in pipeline_def.keys() {
                pipeline_map
                    .entry(pipeline_name.clone())
                    .or_default()
                    .push(skill);
            }
        }
    }

    // Build pipeline info for each detected pipeline
    let mut pipelines: Vec<PipelineInfo> = pipeline_map
        .into_iter()
        .map(|(name, mut pipeline_skills)| {
            // Sort skills by stage and order
            pipeline_skills.sort_by_key(|s| {
                let stage_def = s.frontmatter.pipeline.as_ref().unwrap().get(&name).unwrap();
                (stage_def.stage.clone(), stage_def.order)
            });

            // Group by stage
            let mut stage_map: HashMap<String, Vec<String>> = HashMap::new();
            for skill in &pipeline_skills {
                let stage_def = skill
                    .frontmatter
                    .pipeline
                    .as_ref()
                    .unwrap()
                    .get(&name)
                    .unwrap();
                stage_map
                    .entry(stage_def.stage.clone())
                    .or_default()
                    .push(skill.name.clone());
            }

            let mut stages: Vec<StageInfo> = stage_map
                .into_iter()
                .map(|(stage_name, skills)| StageInfo {
                    name: stage_name,
                    skills,
                })
                .collect();
            stages.sort_by(|a, b| a.name.cmp(&b.name));

            // Detect gaps - skills referenced in after/before but not in pipeline
            let skill_names: HashSet<String> =
                pipeline_skills.iter().map(|s| s.name.clone()).collect();
            let mut gaps = HashSet::new();
            for skill in &pipeline_skills {
                let stage_def = skill
                    .frontmatter
                    .pipeline
                    .as_ref()
                    .unwrap()
                    .get(&name)
                    .unwrap();
                if let Some(after) = &stage_def.after {
                    for dep in after {
                        if !skill_names.contains(dep) {
                            gaps.insert(dep.clone());
                        }
                    }
                }
                if let Some(before) = &stage_def.before {
                    for dep in before {
                        if !skill_names.contains(dep) {
                            gaps.insert(dep.clone());
                        }
                    }
                }
            }

            // Detect cross-pipeline skills
            let cross_pipeline: Vec<String> = pipeline_skills
                .iter()
                .filter(|s| {
                    s.frontmatter
                        .pipeline
                        .as_ref()
                        .map(|p| p.len() > 1)
                        .unwrap_or(false)
                })
                .map(|s| s.name.clone())
                .collect();

            PipelineInfo {
                name,
                stages,
                skills: pipeline_skills.iter().map(|s| s.name.clone()).collect(),
                gaps: gaps.into_iter().collect(),
                cross_pipeline,
            }
        })
        .collect();

    pipelines.sort_by(|a, b| a.name.cmp(&b.name));
    pipelines
}

/// Render the pipeline view
pub fn render(
    f: &mut Frame,
    area: Rect,
    _config: &Config,
    _skills: &[Skill],
    state: &mut PipelineState,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    render_pipeline_list(f, chunks[0], state);
    render_pipeline_detail(f, chunks[1], state);
}

/// Render the pipeline list (left pane)
fn render_pipeline_list(f: &mut Frame, area: Rect, state: &mut PipelineState) {
    let items: Vec<ListItem> = state
        .pipelines
        .iter()
        .map(|p| {
            let has_gaps = !p.gaps.is_empty();
            let (icon, color) = if has_gaps {
                ("⚠", Color::Yellow)
            } else {
                ("●", Color::Green)
            };

            let line = Line::from(vec![
                Span::styled(icon, Style::default().fg(color)),
                Span::raw(" "),
                Span::raw(&p.name),
                Span::raw(" "),
                Span::styled(
                    format!("({} skills)", p.skills.len()),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);

            ListItem::new(line)
        })
        .collect();

    let title = format!(" Pipelines ({}) ", state.pipelines.len());

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

/// Render the pipeline detail pane (right pane)
fn render_pipeline_detail(f: &mut Frame, area: Rect, state: &PipelineState) {
    let content = if let Some(pipeline) = state.selected_pipeline() {
        format_pipeline_detail(pipeline)
    } else if state.pipelines.is_empty() {
        "No pipelines detected.\n\nSkills with pipeline frontmatter will appear here.".to_string()
    } else {
        "No pipeline selected.".to_string()
    };

    let paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .title(" Pipeline Flow ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

/// Format pipeline detail for display
fn format_pipeline_detail(pipeline: &PipelineInfo) -> String {
    let mut lines = Vec::new();

    // Header
    lines.push(format!("═══ {} ═══", pipeline.name.to_uppercase()));
    lines.push(String::new());
    lines.push(format!("Total skills: {}", pipeline.skills.len()));
    lines.push(format!("Stages: {}", pipeline.stages.len()));
    lines.push(String::new());

    // Stage flow
    lines.push("▸ STAGE FLOW:".to_string());
    for (i, stage) in pipeline.stages.iter().enumerate() {
        let stage_num = i + 1;
        lines.push(format!(
            "  {}. {} ({} skills)",
            stage_num,
            stage.name,
            stage.skills.len()
        ));
        for skill in &stage.skills {
            lines.push(format!("     • {}", skill));
        }
        if i < pipeline.stages.len() - 1 {
            lines.push("     ⬇".to_string());
        }
    }
    lines.push(String::new());

    // Cross-pipeline skills
    if !pipeline.cross_pipeline.is_empty() {
        lines.push("▸ CROSS-PIPELINE SKILLS:".to_string());
        lines.push("  Skills used in multiple pipelines:".to_string());
        for skill in &pipeline.cross_pipeline {
            lines.push(format!("  • {}", skill));
        }
        lines.push(String::new());
    }

    // Gaps (missing dependencies)
    if !pipeline.gaps.is_empty() {
        lines.push("▸ GAPS (Missing Dependencies):".to_string());
        lines.push("  Skills referenced but not in pipeline:".to_string());
        for gap in &pipeline.gaps {
            lines.push(format!("  ⚠ {}", gap));
        }
        lines.push(String::new());
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::frontmatter::{Frontmatter, PipelineStage};
    use std::path::PathBuf;

    fn test_skill(name: &str, pipeline_name: &str, stage: &str, order: u32) -> Skill {
        let mut pipeline = HashMap::new();
        pipeline.insert(
            pipeline_name.to_string(),
            PipelineStage {
                stage: stage.to_string(),
                order,
                after: None,
                before: None,
            },
        );

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
                pipeline: Some(pipeline),
            },
        }
    }

    #[test]
    fn should_initialize_with_no_pipelines() {
        // Given / When
        let state = PipelineState::new();

        // Then
        assert_eq!(state.list_state.selected(), Some(0));
        assert!(state.pipelines.is_empty());
    }

    #[test]
    fn should_extract_pipeline_from_skills() {
        // Given
        let skills = vec![
            test_skill("skill-a", "test-pipeline", "first", 1),
            test_skill("skill-b", "test-pipeline", "second", 2),
        ];

        // When
        let pipelines = extract_pipelines(&skills);

        // Then
        assert_eq!(pipelines.len(), 1);
        assert_eq!(pipelines[0].name, "test-pipeline");
        assert_eq!(pipelines[0].skills.len(), 2);
        assert_eq!(pipelines[0].stages.len(), 2);
    }

    #[test]
    fn should_group_skills_by_stage() {
        // Given
        let skills = vec![
            test_skill("skill-a", "test", "init", 1),
            test_skill("skill-b", "test", "init", 2),
            test_skill("skill-c", "test", "process", 1),
        ];

        // When
        let pipelines = extract_pipelines(&skills);

        // Then
        assert_eq!(pipelines[0].stages.len(), 2);
        let init_stage = pipelines[0]
            .stages
            .iter()
            .find(|s| s.name == "init")
            .unwrap();
        assert_eq!(init_stage.skills.len(), 2);
    }

    #[test]
    fn should_move_selection_down() {
        // Given
        let mut state = PipelineState::new();
        state.pipelines = vec![
            PipelineInfo {
                name: "pipeline-a".to_string(),
                stages: vec![],
                skills: vec![],
                gaps: vec![],
                cross_pipeline: vec![],
            },
            PipelineInfo {
                name: "pipeline-b".to_string(),
                stages: vec![],
                skills: vec![],
                gaps: vec![],
                cross_pipeline: vec![],
            },
        ];

        // When
        state.next();

        // Then
        assert_eq!(state.list_state.selected(), Some(1));
    }

    #[test]
    fn should_wrap_selection_at_end() {
        // Given
        let mut state = PipelineState::new();
        state.pipelines = vec![PipelineInfo {
            name: "pipeline-a".to_string(),
            stages: vec![],
            skills: vec![],
            gaps: vec![],
            cross_pipeline: vec![],
        }];
        state.list_state.select(Some(0));

        // When
        state.next();

        // Then (should wrap to first)
        assert_eq!(state.list_state.selected(), Some(0));
    }

    #[test]
    fn should_move_selection_up() {
        // Given
        let mut state = PipelineState::new();
        state.pipelines = vec![
            PipelineInfo {
                name: "pipeline-a".to_string(),
                stages: vec![],
                skills: vec![],
                gaps: vec![],
                cross_pipeline: vec![],
            },
            PipelineInfo {
                name: "pipeline-b".to_string(),
                stages: vec![],
                skills: vec![],
                gaps: vec![],
                cross_pipeline: vec![],
            },
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
        let mut state = PipelineState::new();
        state.pipelines = vec![
            PipelineInfo {
                name: "pipeline-a".to_string(),
                stages: vec![],
                skills: vec![],
                gaps: vec![],
                cross_pipeline: vec![],
            },
            PipelineInfo {
                name: "pipeline-b".to_string(),
                stages: vec![],
                skills: vec![],
                gaps: vec![],
                cross_pipeline: vec![],
            },
        ];
        state.list_state.select(Some(0));

        // When
        state.previous();

        // Then (should wrap to last)
        assert_eq!(state.list_state.selected(), Some(1));
    }
}
