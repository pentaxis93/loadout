use anyhow::Result;
use colored::Colorize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

use crate::config::Config;
use crate::skill::{self, Skill};

const MARKER_FILE: &str = ".managed-by-loadout";

const PLACEHOLDER_DESCRIPTIONS: &[&str] = &["Description here", "TODO", "TBD", "FIXME"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

impl Severity {
    pub fn color(&self) -> colored::Color {
        match self {
            Severity::Error => colored::Color::Red,
            Severity::Warning => colored::Color::Yellow,
            Severity::Info => colored::Color::Blue,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Severity::Error => "ERROR",
            Severity::Warning => "WARN",
            Severity::Info => "INFO",
        }
    }
}

#[derive(Debug)]
pub struct Finding {
    pub severity: Severity,
    pub message: String,
    pub fix: String,
    pub path: Option<PathBuf>,
    /// Key for suppression matching: "check-type:source:detail"
    pub suppress_key: String,
}

impl Finding {
    fn error(message: impl Into<String>, fix: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            fix: fix.into(),
            path: None,
            suppress_key: key.into(),
        }
    }

    fn error_with_path(
        message: impl Into<String>,
        fix: impl Into<String>,
        key: impl Into<String>,
        path: PathBuf,
    ) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            fix: fix.into(),
            path: Some(path),
            suppress_key: key.into(),
        }
    }

    fn warning(message: impl Into<String>, fix: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            fix: fix.into(),
            path: None,
            suppress_key: key.into(),
        }
    }

    fn warning_with_path(
        message: impl Into<String>,
        fix: impl Into<String>,
        key: impl Into<String>,
        path: PathBuf,
    ) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            fix: fix.into(),
            path: Some(path),
            suppress_key: key.into(),
        }
    }

    fn info(message: impl Into<String>, fix: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            severity: Severity::Info,
            message: message.into(),
            fix: fix.into(),
            path: None,
            suppress_key: key.into(),
        }
    }
}

pub fn check(
    config: &Config,
    filter_severity: Option<Severity>,
    verbose: bool,
) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();

    // Discover all skills across all sources
    let all_skills = skill::discover_all(&config.sources.skills)?;
    let skill_map: HashMap<String, &Skill> = all_skills
        .iter()
        .map(|s| (s.frontmatter.name.clone(), s))
        .collect();

    // Build set of known skill names for filtering
    let known_skills: HashSet<String> = all_skills.iter().map(|s| s.name.clone()).collect();

    // Extract cross-references from all skills
    let mut crossrefs: HashMap<String, Vec<skill::CrossRef>> = HashMap::new();
    for skill in &all_skills {
        let skill_md = skill.path.join("SKILL.md");
        let content = fs::read_to_string(&skill_md)?;
        let refs =
            skill::extract_references_with_filter(&content, &skill.name, Some(&known_skills));
        if !refs.is_empty() {
            crossrefs.insert(skill.name.clone(), refs);
        }
    }

    // Check 1: Dangling references
    findings.extend(check_dangling_references(&crossrefs, &skill_map));

    // Check 2: Orphaned skills
    findings.extend(check_orphaned_skills(config, &all_skills));

    // Check 3: Name/directory mismatches
    findings.extend(check_name_directory_mismatch(&all_skills));

    // Check 4: Missing required frontmatter fields
    findings.extend(check_missing_frontmatter(&all_skills));

    // Check 5: Broken symlinks in target directories
    findings.extend(check_broken_symlinks(config)?);

    // Check 6: Unmanaged conflicts in target directories
    findings.extend(check_unmanaged_conflicts(config)?);

    // Check 7: Empty or placeholder descriptions
    findings.extend(check_placeholder_descriptions(&all_skills));

    // Check 8: Pipeline integrity
    findings.extend(check_pipeline_integrity(&all_skills, &known_skills));

    // Check 9: Untagged/unpipelined skills
    findings.extend(check_missing_metadata(&all_skills));

    // Sort by severity (errors first)
    findings.sort_by_key(|f| f.severity);
    findings.reverse(); // Reverse to get errors first

    // Filter by severity if requested
    if let Some(min_severity) = filter_severity {
        findings.retain(|f| f.severity >= min_severity);
    }

    // Apply suppression
    let ignore_set: HashSet<&str> = config.check.ignore.iter().map(|s| s.as_str()).collect();
    if !verbose {
        findings.retain(|f| !ignore_set.contains(f.suppress_key.as_str()));
    } else {
        // In verbose mode, mark suppressed findings but keep them
        for finding in &mut findings {
            if ignore_set.contains(finding.suppress_key.as_str()) {
                finding.message = format!("{} (suppressed)", finding.message);
            }
        }
    }

    Ok(findings)
}

fn check_dangling_references(
    crossrefs: &HashMap<String, Vec<skill::CrossRef>>,
    skill_map: &HashMap<String, &Skill>,
) -> Vec<Finding> {
    let mut findings = Vec::new();

    for (source_skill, refs) in crossrefs {
        for crossref in refs {
            if !skill_map.contains_key(&crossref.target) {
                findings.push(Finding::error(
                    format!(
                        "Skill '{}' references non-existent skill '{}' (line {})",
                        source_skill, crossref.target, crossref.line
                    ),
                    format!(
                        "Create the skill with `loadout new {}`, or remove the reference at line {}",
                        crossref.target, crossref.line
                    ),
                    format!("dangling:{}:{}", source_skill, crossref.target),
                ));
            }
        }
    }

    findings
}

fn check_orphaned_skills(config: &Config, all_skills: &[Skill]) -> Vec<Finding> {
    let mut findings = Vec::new();

    // Collect all skill names mentioned in config
    let mut mentioned_skills: HashSet<String> = HashSet::new();
    mentioned_skills.extend(config.global.skills.iter().cloned());

    for project in config.projects.values() {
        mentioned_skills.extend(project.skills.iter().cloned());
    }

    // Check for skills in sources but not in config
    for skill in all_skills {
        if !mentioned_skills.contains(&skill.name) {
            findings.push(Finding::warning_with_path(
                format!(
                    "Skill '{}' exists in sources but not in any config section",
                    skill.name
                ),
                format!("Add '{}' to [global].skills in loadout.toml", skill.name),
                format!("orphaned:{}", skill.name),
                skill.path.clone(),
            ));
        }
    }

    findings
}

fn check_name_directory_mismatch(all_skills: &[Skill]) -> Vec<Finding> {
    let mut findings = Vec::new();

    for skill in all_skills {
        if let Some(dir_name) = skill.path.file_name() {
            if dir_name != skill.name.as_str() {
                findings.push(Finding::error_with_path(
                    format!(
                        "Skill name '{}' does not match directory name '{}'",
                        skill.name,
                        dir_name.to_string_lossy()
                    ),
                    format!(
                        "Rename directory to '{}' or update frontmatter name field",
                        skill.name
                    ),
                    format!("name-mismatch:{}", skill.name),
                    skill.path.clone(),
                ));
            }
        }
    }

    findings
}

fn check_missing_frontmatter(all_skills: &[Skill]) -> Vec<Finding> {
    let mut findings = Vec::new();

    for skill in all_skills {
        if skill.frontmatter.description.is_empty() {
            findings.push(Finding::error_with_path(
                format!("Skill '{}' has empty description", skill.name),
                "Add a description to the SKILL.md frontmatter".to_string(),
                format!("empty-description:{}", skill.name),
                skill.path.clone(),
            ));
        }
    }

    findings
}

fn check_broken_symlinks(config: &Config) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();

    let mut all_targets = config.global.targets.clone();
    for project_path in config.projects.keys() {
        all_targets.push(project_path.join(".claude/skills"));
        all_targets.push(project_path.join(".opencode/skills"));
        all_targets.push(project_path.join(".agents/skills"));
    }

    for target in &all_targets {
        if !target.exists() {
            continue;
        }

        for entry in fs::read_dir(target)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_symlink() && fs::metadata(&path).is_err() {
                findings.push(Finding::error_with_path(
                    "Broken symlink: target does not exist".to_string(),
                    "Run `loadout clean && loadout install` to rebuild symlinks".to_string(),
                    format!(
                        "broken-symlink:{}",
                        path.file_name().unwrap_or_default().to_string_lossy()
                    ),
                    path,
                ));
            }
        }
    }

    Ok(findings)
}

fn check_unmanaged_conflicts(config: &Config) -> Result<Vec<Finding>> {
    let mut findings = Vec::new();

    let mut all_targets = config.global.targets.clone();
    for project_path in config.projects.keys() {
        all_targets.push(project_path.join(".claude/skills"));
        all_targets.push(project_path.join(".opencode/skills"));
        all_targets.push(project_path.join(".agents/skills"));
    }

    for target in &all_targets {
        if !target.exists() {
            continue;
        }

        for entry in fs::read_dir(target)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_symlink() {
                continue;
            }

            if path.is_dir() {
                let marker_path = path.join(MARKER_FILE);
                if !marker_path.exists() {
                    findings.push(Finding::warning_with_path(
                        "Unmanaged directory conflicts with skill slot".to_string(),
                        "Remove the directory, or let loadout manage it with `loadout install`"
                            .to_string(),
                        format!(
                            "unmanaged:{}",
                            path.file_name().unwrap_or_default().to_string_lossy()
                        ),
                        path,
                    ));
                }
            }
        }
    }

    Ok(findings)
}

fn check_placeholder_descriptions(all_skills: &[Skill]) -> Vec<Finding> {
    let mut findings = Vec::new();

    for skill in all_skills {
        let desc = &skill.frontmatter.description;

        if PLACEHOLDER_DESCRIPTIONS.iter().any(|p| desc.contains(p)) {
            findings.push(Finding::warning_with_path(
                format!(
                    "Skill '{}' has placeholder description: '{}'",
                    skill.name,
                    desc.chars().take(50).collect::<String>()
                ),
                format!(
                    "Edit {}/SKILL.md and write a real description",
                    skill.path.display()
                ),
                format!("placeholder:{}", skill.name),
                skill.path.clone(),
            ));
        } else if desc.len() < 10 {
            findings.push(Finding::warning_with_path(
                format!(
                    "Skill '{}' has very short description ({} chars): '{}'",
                    skill.name,
                    desc.len(),
                    desc
                ),
                format!(
                    "Edit {}/SKILL.md and expand the description",
                    skill.path.display()
                ),
                format!("short-description:{}", skill.name),
                skill.path.clone(),
            ));
        }
    }

    findings
}

fn check_pipeline_integrity(all_skills: &[Skill], known_skills: &HashSet<String>) -> Vec<Finding> {
    let mut findings = Vec::new();

    // Build a map of pipeline declarations: pipeline_name -> skill_name -> PipelineStage
    let mut pipeline_map: HashMap<String, HashMap<String, &skill::PipelineStage>> = HashMap::new();
    for skill in all_skills {
        if let Some(pipeline) = &skill.frontmatter.pipeline {
            for (name, stage) in pipeline {
                pipeline_map
                    .entry(name.clone())
                    .or_default()
                    .insert(skill.name.clone(), stage);
            }
        }
    }

    // Check each pipeline for integrity
    for (pipeline_name, stages) in &pipeline_map {
        for (skill_name, stage) in stages {
            // Check after references exist
            if let Some(after) = &stage.after {
                for dep in after {
                    if !known_skills.contains(dep) {
                        findings.push(Finding::error(
                            format!(
                                "Pipeline '{}': skill '{}' declares after: ['{}'] but skill doesn't exist",
                                pipeline_name, skill_name, dep
                            ),
                            format!(
                                "Create the skill with `loadout new {}`, or remove it from the after list",
                                dep
                            ),
                            format!("pipeline-missing:{}:{}:{}", pipeline_name, skill_name, dep),
                        ));
                    }
                }
            }

            // Check before references exist
            if let Some(before) = &stage.before {
                for dep in before {
                    if !known_skills.contains(dep) {
                        findings.push(Finding::error(
                            format!(
                                "Pipeline '{}': skill '{}' declares before: ['{}'] but skill doesn't exist",
                                pipeline_name, skill_name, dep
                            ),
                            format!(
                                "Create the skill with `loadout new {}`, or remove it from the before list",
                                dep
                            ),
                            format!("pipeline-missing:{}:{}:{}", pipeline_name, skill_name, dep),
                        ));
                    }
                }
            }

            // Check for asymmetric after/before declarations
            if let Some(after) = &stage.after {
                for dep in after {
                    if let Some(dep_stage) = stages.get(dep) {
                        // dep should have before: [skill_name]
                        let has_reciprocal = dep_stage
                            .before
                            .as_ref()
                            .map(|b| b.contains(skill_name))
                            .unwrap_or(false);
                        if !has_reciprocal {
                            findings.push(Finding::warning(
                                format!(
                                    "Pipeline '{}': '{}' declares after: ['{}'] but '{}' doesn't declare before: ['{}']",
                                    pipeline_name, skill_name, dep, dep, skill_name
                                ),
                                format!(
                                    "Add before: ['{}'] to skill '{}' in pipeline '{}'",
                                    skill_name, dep, pipeline_name
                                ),
                                format!("pipeline-gap:{}:{}:{}", pipeline_name, skill_name, dep),
                            ));
                        }
                    }
                }
            }
        }
    }

    findings
}

fn check_missing_metadata(all_skills: &[Skill]) -> Vec<Finding> {
    // Only check when the library is partially annotated — at least one skill
    // has tags or pipeline. This avoids noise for users who haven't adopted
    // the metadata scheme.
    let any_annotated = all_skills.iter().any(|s| {
        s.frontmatter
            .tags
            .as_ref()
            .map(|t| !t.is_empty())
            .unwrap_or(false)
            || s.frontmatter.pipeline.is_some()
    });

    if !any_annotated {
        return Vec::new();
    }

    let mut findings = Vec::new();

    for skill in all_skills {
        let has_tags = skill
            .frontmatter
            .tags
            .as_ref()
            .map(|t| !t.is_empty())
            .unwrap_or(false);
        let has_pipeline = skill.frontmatter.pipeline.is_some();

        if !has_tags && !has_pipeline {
            findings.push(Finding::info(
                format!(
                    "Skill '{}' has no tags and isn't in any pipeline",
                    skill.name
                ),
                format!(
                    "Add tags: [<tag>] or pipeline metadata to {}/SKILL.md",
                    skill.path.display()
                ),
                format!("no-metadata:{}", skill.name),
            ));
        }
    }

    findings
}

pub fn print_findings(findings: &[Finding]) {
    if findings.is_empty() {
        println!("{}", "No issues found.".green());
        return;
    }

    // Group by severity
    let mut by_severity: HashMap<Severity, Vec<&Finding>> = HashMap::new();
    for finding in findings {
        by_severity
            .entry(finding.severity)
            .or_default()
            .push(finding);
    }

    // Print in order: Error -> Warning -> Info
    for severity in [Severity::Error, Severity::Warning, Severity::Info] {
        if let Some(findings) = by_severity.get(&severity) {
            println!(
                "\n{} ({} found)",
                severity.label().color(severity.color()).bold(),
                findings.len()
            );

            for finding in findings {
                if let Some(path) = &finding.path {
                    println!(
                        "  {} {}",
                        "•".color(severity.color()),
                        format!("{} ({})", finding.message, path.display()).dimmed()
                    );
                } else {
                    println!(
                        "  {} {}",
                        "•".color(severity.color()),
                        finding.message.dimmed()
                    );
                }
                println!(
                    "    {} {}",
                    "↳".color(severity.color()),
                    finding.fix.dimmed()
                );
            }
        }
    }

    println!();
}

pub fn exit_code(findings: &[Finding]) -> i32 {
    if findings.iter().any(|f| f.severity == Severity::Error) {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create a test skill
    fn test_skill(name: &str, description: &str) -> Skill {
        use crate::skill::frontmatter::Frontmatter;

        Skill {
            name: name.to_string(),
            path: PathBuf::from(format!("/test/skills/{}", name)),
            skill_file: PathBuf::from(format!("/test/skills/{}/SKILL.md", name)),
            frontmatter: Frontmatter {
                name: name.to_string(),
                description: description.to_string(),
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
                tags: None,
                pipeline: None,
            },
        }
    }

    #[test]
    fn should_detect_dangling_references() {
        // Given
        let mut crossrefs = HashMap::new();
        crossrefs.insert(
            "skill-a".to_string(),
            vec![skill::CrossRef {
                target: "nonexistent".to_string(),
                line: 10,
                method: skill::DetectionMethod::XmlCrossref,
            }],
        );

        let skill_map: HashMap<String, &Skill> = HashMap::new();

        // When
        let findings = check_dangling_references(&crossrefs, &skill_map);

        // Then
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Error);
        assert!(findings[0].message.contains("nonexistent"));
        assert!(findings[0].fix.contains("loadout new nonexistent"));
    }

    #[test]
    fn should_detect_orphaned_skills() {
        // Given
        let config = Config {
            sources: crate::config::Sources {
                skills: vec![PathBuf::from("/test/skills")],
            },
            global: crate::config::Global {
                targets: vec![],
                skills: vec!["skill-a".to_string()],
            },
            projects: HashMap::new(),
            check: Default::default(),
        };

        let skills = vec![
            test_skill("skill-a", "Active skill"),
            test_skill("skill-b", "Orphaned skill"),
        ];

        // When
        let findings = check_orphaned_skills(&config, &skills);

        // Then
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Warning);
        assert!(findings[0].message.contains("skill-b"));
        assert!(findings[0].fix.contains("loadout.toml"));
    }

    #[test]
    fn should_detect_placeholder_descriptions() {
        // Given
        let skills = vec![
            test_skill("skill-a", "TODO: write description"),
            test_skill("skill-b", "Short"),
            test_skill("skill-c", "This is a proper description"),
        ];

        // When
        let findings = check_placeholder_descriptions(&skills);

        // Then
        assert_eq!(findings.len(), 2);
        assert!(findings.iter().any(|f| f.message.contains("skill-a")));
        assert!(findings.iter().any(|f| f.message.contains("skill-b")));
    }

    #[test]
    fn should_detect_pipeline_integrity_issues() {
        // Given: skill-a declares after: [skill-b] but skill-b doesn't declare before: [skill-a]
        use crate::skill::frontmatter::{Frontmatter, PipelineStage};

        let skills = vec![
            Skill {
                name: "skill-a".to_string(),
                path: PathBuf::from("/test/skills/skill-a"),
                skill_file: PathBuf::from("/test/skills/skill-a/SKILL.md"),
                frontmatter: Frontmatter {
                    name: "skill-a".to_string(),
                    description: "Test A".to_string(),
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
                    tags: None,
                    pipeline: Some({
                        let mut m = HashMap::new();
                        m.insert(
                            "test-pipeline".to_string(),
                            PipelineStage {
                                stage: "second".to_string(),
                                order: 2,
                                after: Some(vec!["skill-b".to_string()]),
                                before: None,
                            },
                        );
                        m
                    }),
                },
            },
            Skill {
                name: "skill-b".to_string(),
                path: PathBuf::from("/test/skills/skill-b"),
                skill_file: PathBuf::from("/test/skills/skill-b/SKILL.md"),
                frontmatter: Frontmatter {
                    name: "skill-b".to_string(),
                    description: "Test B".to_string(),
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
                    tags: None,
                    pipeline: Some({
                        let mut m = HashMap::new();
                        m.insert(
                            "test-pipeline".to_string(),
                            PipelineStage {
                                stage: "first".to_string(),
                                order: 1,
                                after: None,
                                before: None, // Missing before: [skill-a]
                            },
                        );
                        m
                    }),
                },
            },
        ];

        let known_skills: HashSet<String> = skills.iter().map(|s| s.name.clone()).collect();

        // When
        let findings = check_pipeline_integrity(&skills, &known_skills);

        // Then
        assert!(findings.iter().any(|f| {
            f.severity == Severity::Warning && f.message.contains("doesn't declare before")
        }));
    }

    #[test]
    fn should_detect_missing_metadata_when_library_is_partially_annotated() {
        // Given: one tagged skill and one with no metadata
        use crate::skill::frontmatter::Frontmatter;

        let tagged_skill = Skill {
            name: "tagged-skill".to_string(),
            path: PathBuf::from("/test/skills/tagged-skill"),
            skill_file: PathBuf::from("/test/skills/tagged-skill/SKILL.md"),
            frontmatter: Frontmatter {
                name: "tagged-skill".to_string(),
                description: "Has tags".to_string(),
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
                tags: Some(vec!["example".to_string()]),
                pipeline: None,
            },
        };

        let skills = vec![
            tagged_skill,
            test_skill("lonely-skill", "No metadata at all"),
        ];

        // When
        let findings = check_missing_metadata(&skills);

        // Then
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Info);
        assert!(findings[0].message.contains("lonely-skill"));
    }

    #[test]
    fn should_skip_missing_metadata_check_when_no_skills_are_annotated() {
        // Given: all skills lack tags and pipeline
        let skills = vec![
            test_skill("skill-a", "No metadata"),
            test_skill("skill-b", "Also no metadata"),
        ];

        // When
        let findings = check_missing_metadata(&skills);

        // Then: no findings — the library hasn't adopted metadata
        assert!(findings.is_empty());
    }

    #[test]
    fn should_include_fix_suggestions_in_all_findings() {
        // Given
        let mut crossrefs = HashMap::new();
        crossrefs.insert(
            "skill-a".to_string(),
            vec![skill::CrossRef {
                target: "missing".to_string(),
                line: 5,
                method: skill::DetectionMethod::XmlCrossref,
            }],
        );

        let skill_map: HashMap<String, &Skill> = HashMap::new();

        // When
        let findings = check_dangling_references(&crossrefs, &skill_map);

        // Then: every finding has a non-empty fix
        for finding in &findings {
            assert!(
                !finding.fix.is_empty(),
                "Finding should have a fix suggestion"
            );
        }
    }

    #[test]
    fn should_determine_exit_code_from_severity() {
        // Given
        let findings_with_errors = vec![Finding::error("Something failed", "Fix it", "test:error")];
        let findings_warnings_only = vec![Finding::warning(
            "Something suspicious",
            "Check it",
            "test:warning",
        )];
        let no_findings = vec![];

        // When/Then
        assert_eq!(exit_code(&findings_with_errors), 1);
        assert_eq!(exit_code(&findings_warnings_only), 0);
        assert_eq!(exit_code(&no_findings), 0);
    }
}
