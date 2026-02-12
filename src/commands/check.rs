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
    pub path: Option<PathBuf>,
}

impl Finding {
    fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            path: None,
        }
    }

    fn error_with_path(message: impl Into<String>, path: PathBuf) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            path: Some(path),
        }
    }

    #[cfg(test)]
    fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            path: None,
        }
    }

    fn warning_with_path(message: impl Into<String>, path: PathBuf) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            path: Some(path),
        }
    }

    fn info(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Info,
            message: message.into(),
            path: None,
        }
    }
}

pub fn check(config: &Config, filter_severity: Option<Severity>) -> Result<Vec<Finding>> {
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

    // Check 3: Name/directory mismatches (already validated in frontmatter, but check again)
    findings.extend(check_name_directory_mismatch(&all_skills));

    // Check 4: Missing required frontmatter fields (also validated, but double-check)
    findings.extend(check_missing_frontmatter(&all_skills));

    // Check 5: Broken symlinks in target directories
    findings.extend(check_broken_symlinks(config)?);

    // Check 6: Unmanaged conflicts in target directories
    findings.extend(check_unmanaged_conflicts(config)?);

    // Check 7: Empty or placeholder descriptions
    findings.extend(check_placeholder_descriptions(&all_skills));

    // Check 8: Circular references
    findings.extend(check_circular_references(&crossrefs));

    // Sort by severity (errors first)
    findings.sort_by_key(|f| f.severity);
    findings.reverse(); // Reverse to get errors first

    // Filter by severity if requested
    if let Some(min_severity) = filter_severity {
        findings.retain(|f| f.severity >= min_severity);
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
                findings.push(Finding::error(format!(
                    "Skill '{}' references non-existent skill '{}' (line {})",
                    source_skill, crossref.target, crossref.line
                )));
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
        // Name and description are validated when parsing frontmatter
        // This check is mostly redundant but included for completeness
        if skill.frontmatter.description.is_empty() {
            findings.push(Finding::error_with_path(
                format!("Skill '{}' has empty description", skill.name),
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
        // Project-local targets: .claude/skills, .opencode/skills, .agents/skills
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
                // Check if symlink target exists
                if fs::metadata(&path).is_err() {
                    findings.push(Finding::error_with_path(
                        format!("Broken symlink: target does not exist"),
                        path,
                    ));
                }
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

            // Skip symlinks (those are managed)
            if path.is_symlink() {
                continue;
            }

            // Check if it's a directory without our marker
            if path.is_dir() {
                let marker_path = path.join(MARKER_FILE);
                if !marker_path.exists() {
                    findings.push(Finding::warning_with_path(
                        format!("Unmanaged directory conflicts with skill slot"),
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

        // Check for placeholder text
        if PLACEHOLDER_DESCRIPTIONS.iter().any(|p| desc.contains(p)) {
            findings.push(Finding::warning_with_path(
                format!(
                    "Skill '{}' has placeholder description: '{}'",
                    skill.name,
                    desc.chars().take(50).collect::<String>()
                ),
                skill.path.clone(),
            ));
        }
        // Check for very short descriptions (<10 chars)
        else if desc.len() < 10 {
            findings.push(Finding::warning_with_path(
                format!(
                    "Skill '{}' has very short description ({} chars): '{}'",
                    skill.name,
                    desc.len(),
                    desc
                ),
                skill.path.clone(),
            ));
        }
    }

    findings
}

fn check_circular_references(crossrefs: &HashMap<String, Vec<skill::CrossRef>>) -> Vec<Finding> {
    let mut findings = Vec::new();

    // Build adjacency map
    let mut graph: HashMap<String, HashSet<String>> = HashMap::new();
    for (source, refs) in crossrefs {
        let targets: HashSet<String> = refs.iter().map(|r| r.target.clone()).collect();
        graph.insert(source.clone(), targets);
    }

    // Detect cycles using DFS
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();

    for skill in graph.keys() {
        if !visited.contains(skill) {
            if let Some(cycle) = find_cycle(skill, &graph, &mut visited, &mut rec_stack) {
                findings.push(Finding::info(format!(
                    "Circular reference detected: {}",
                    cycle.join(" → ")
                )));
            }
        }
    }

    findings
}

fn find_cycle(
    node: &str,
    graph: &HashMap<String, HashSet<String>>,
    visited: &mut HashSet<String>,
    rec_stack: &mut HashSet<String>,
) -> Option<Vec<String>> {
    visited.insert(node.to_string());
    rec_stack.insert(node.to_string());

    if let Some(neighbors) = graph.get(node) {
        for neighbor in neighbors {
            if !visited.contains(neighbor) {
                if let Some(mut cycle) = find_cycle(neighbor, graph, visited, rec_stack) {
                    cycle.insert(0, node.to_string());
                    return Some(cycle);
                }
            } else if rec_stack.contains(neighbor) {
                // Found a cycle
                return Some(vec![node.to_string(), neighbor.clone()]);
            }
        }
    }

    rec_stack.remove(node);
    None
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

    // Print in order: Error → Warning → Info
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
    fn should_detect_circular_references() {
        // Given: skill-a → skill-b → skill-a (cycle)
        let mut crossrefs = HashMap::new();
        crossrefs.insert(
            "skill-a".to_string(),
            vec![skill::CrossRef {
                target: "skill-b".to_string(),
                line: 1,
                method: skill::DetectionMethod::XmlCrossref,
            }],
        );
        crossrefs.insert(
            "skill-b".to_string(),
            vec![skill::CrossRef {
                target: "skill-a".to_string(),
                line: 1,
                method: skill::DetectionMethod::XmlCrossref,
            }],
        );

        // When
        let findings = check_circular_references(&crossrefs);

        // Then
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Info);
        assert!(findings[0].message.contains("Circular reference"));
    }

    #[test]
    fn should_determine_exit_code_from_severity() {
        // Given
        let findings_with_errors = vec![Finding::error("Something failed")];
        let findings_warnings_only = vec![Finding::warning("Something suspicious")];
        let no_findings = vec![];

        // When/Then
        assert_eq!(exit_code(&findings_with_errors), 1);
        assert_eq!(exit_code(&findings_warnings_only), 0);
        assert_eq!(exit_code(&no_findings), 0);
    }
}
