//! List command implementation

use anyhow::{Context, Result};
use colored::Colorize;
use std::collections::{HashMap, HashSet};
use std::fs;

use crate::config::Config;
use crate::skill;

pub enum ListMode {
    Default,
    Groups,
    Refs(String),
    Missing,
    Tags,
    Tag(String),
    Pipelines,
    Pipeline(String),
}

/// List enabled skills per scope
pub fn list(config: &Config, mode: ListMode) -> Result<()> {
    match mode {
        ListMode::Default => list_default(config),
        ListMode::Groups => list_groups(config),
        ListMode::Refs(skill_name) => list_refs(config, &skill_name),
        ListMode::Missing => list_missing(config),
        ListMode::Tags => list_tags(config),
        ListMode::Tag(tag) => list_by_tag(config, &tag),
        ListMode::Pipelines => list_pipelines(config),
        ListMode::Pipeline(name) => list_pipeline(config, &name),
    }
}

fn list_default(config: &Config) -> Result<()> {
    // Discover all available skills
    let skills = skill::discover_all(&config.sources.skills)?;
    let skill_map = skill::build_skill_map(skills);

    // List global skills
    println!("{}", "--- Global scope ---".cyan().bold());
    println!("Skills: {}", config.global.skills.len());
    for skill_name in &config.global.skills {
        if let Some(skill) = skill_map.get(skill_name) {
            println!(
                "  {} {} ({})",
                "✓".green(),
                skill_name,
                skill.path.display().to_string().dimmed()
            );
        } else {
            println!("  {} {} {}", "✗".red(), skill_name, "(not found)".red());
        }
    }

    // List project skills
    for (project_path, project_config) in &config.projects {
        println!();
        println!(
            "{} {}",
            "--- Project:".cyan().bold(),
            project_path.display()
        );

        let mut all_skills = Vec::new();

        // Add global skills if inherited
        if project_config.inherit {
            all_skills.extend(config.global.skills.clone());
        }

        // Add project-specific skills
        all_skills.extend(project_config.skills.clone());

        // Deduplicate
        all_skills.sort();
        all_skills.dedup();

        println!(
            "Skills: {} (inherit: {})",
            all_skills.len(),
            if project_config.inherit {
                "true"
            } else {
                "false"
            }
        );

        for skill_name in &all_skills {
            if let Some(skill) = skill_map.get(skill_name) {
                let source = if config.global.skills.contains(skill_name) {
                    "global".dimmed()
                } else {
                    "project".dimmed()
                };
                println!(
                    "  {} {} ({}, {})",
                    "✓".green(),
                    skill_name,
                    source,
                    skill.path.display().to_string().dimmed()
                );
            } else {
                println!("  {} {} {}", "✗".red(), skill_name, "(not found)".red());
            }
        }
    }

    Ok(())
}

#[cfg(feature = "graph")]
fn list_groups(config: &Config) -> Result<()> {
    use crate::graph::SkillGraph;

    let skills = skill::discover_all(&config.sources.skills)?;
    let known_skills: HashSet<String> = skills.iter().map(|s| s.name.clone()).collect();
    let mut crossrefs = HashMap::new();

    for skill in &skills {
        let skill_md = skill.path.join("SKILL.md");
        let content = fs::read_to_string(&skill_md)?;
        let refs =
            skill::extract_references_with_filter(&content, &skill.name, Some(&known_skills));
        if !refs.is_empty() {
            crossrefs.insert(skill.name.clone(), refs);
        }
    }

    let graph = SkillGraph::from_crossrefs(&crossrefs);

    println!("{}", "--- Skills by cluster ---".cyan().bold());

    if graph.clusters.is_empty() {
        println!(
            "{}",
            "No clusters detected (no circular references)".dimmed()
        );
        println!("\nShowing all skills:");
        let mut all_names: Vec<_> = skills.iter().map(|s| &s.name).collect();
        all_names.sort();
        for name in all_names {
            println!("  • {}", name);
        }
    } else {
        for (i, cluster) in graph.clusters.iter().enumerate() {
            println!(
                "\n{} {}",
                format!("Cluster {}:", i + 1).yellow().bold(),
                format!("({} skills)", cluster.len()).dimmed()
            );
            for skill in cluster {
                println!("  • {}", skill);
            }
        }

        // Show unclustered skills
        let clustered: HashSet<_> = graph.clusters.iter().flat_map(|c| c.iter()).collect();
        let unclustered: Vec<_> = skills
            .iter()
            .filter(|s| !clustered.contains(&&s.name))
            .map(|s| &s.name)
            .collect();

        if !unclustered.is_empty() {
            println!("\n{}", "Unclustered skills:".dimmed());
            for skill in unclustered {
                println!("  • {}", skill);
            }
        }
    }

    Ok(())
}

#[cfg(not(feature = "graph"))]
fn list_groups(config: &Config) -> Result<()> {
    let skills = skill::discover_all(&config.sources.skills)?;

    println!(
        "{}",
        "--- Skills (cluster detection unavailable) ---"
            .cyan()
            .bold()
    );
    println!(
        "{}",
        "Note: Install with --features graph for cluster detection\n".yellow()
    );

    let mut all_names: Vec<_> = skills.iter().map(|s| &s.name).collect();
    all_names.sort();
    for name in all_names {
        println!("  • {}", name);
    }

    Ok(())
}

fn list_refs(config: &Config, skill_name: &str) -> Result<()> {
    let skills = skill::discover_all(&config.sources.skills)?;
    let skill_map = skill::build_skill_map(skills.clone());

    // Check if skill exists
    if !skill_map.contains_key(skill_name) {
        anyhow::bail!("Skill '{}' not found in any source", skill_name);
    }

    // Extract all cross-references
    let known_skills: HashSet<String> = skills.iter().map(|s| s.name.clone()).collect();
    let mut crossrefs: HashMap<String, Vec<skill::CrossRef>> = HashMap::new();
    for skill in &skills {
        let skill_md = skill.path.join("SKILL.md");
        let content = fs::read_to_string(&skill_md)?;
        let refs =
            skill::extract_references_with_filter(&content, &skill.name, Some(&known_skills));
        if !refs.is_empty() {
            crossrefs.insert(skill.name.clone(), refs);
        }
    }

    // Find outgoing references (skills this skill references)
    let outgoing: Vec<String> = crossrefs
        .get(skill_name)
        .map(|refs| refs.iter().map(|r| r.target.clone()).collect())
        .unwrap_or_default();

    // Find incoming references (skills that reference this skill)
    let incoming: Vec<String> = crossrefs
        .iter()
        .filter(|(_, refs)| refs.iter().any(|r| r.target == skill_name))
        .map(|(name, _)| name.clone())
        .collect();

    println!(
        "{} {}",
        "--- References for".cyan().bold(),
        skill_name.cyan().bold()
    );

    println!("\n{} ({})", "Outgoing:".yellow(), outgoing.len());
    if outgoing.is_empty() {
        println!("  {}", "(none)".dimmed());
    } else {
        for target in &outgoing {
            println!("  → {}", target);
        }
    }

    println!("\n{} ({})", "Incoming:".green(), incoming.len());
    if incoming.is_empty() {
        println!("  {}", "(none)".dimmed());
    } else {
        for source in &incoming {
            println!("  ← {}", source);
        }
    }

    Ok(())
}

fn list_tags(config: &Config) -> Result<()> {
    let skills = skill::discover_all(&config.sources.skills)?;

    // Collect tag counts
    let mut tag_counts: HashMap<String, Vec<String>> = HashMap::new();
    for s in &skills {
        if let Some(tags) = &s.frontmatter.tags {
            for tag in tags {
                tag_counts
                    .entry(tag.clone())
                    .or_default()
                    .push(s.name.clone());
            }
        }
    }

    if tag_counts.is_empty() {
        println!(
            "{}",
            "No tags found. Add tags to SKILL.md frontmatter.".dimmed()
        );
        return Ok(());
    }

    // Sort by count descending, then by name
    let mut tags: Vec<_> = tag_counts.iter().collect();
    tags.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(b.0)));

    println!("{}", "--- Tags ---".cyan().bold());
    println!();
    for (tag, skills) in &tags {
        println!(
            "  {} {} {}",
            tag.yellow(),
            format!("({})", skills.len()).dimmed(),
            skills.join(", ").dimmed()
        );
    }

    // Count untagged
    let untagged: Vec<_> = skills
        .iter()
        .filter(|s| s.frontmatter.tags.is_none() || s.frontmatter.tags.as_ref().unwrap().is_empty())
        .map(|s| s.name.as_str())
        .collect();

    if !untagged.is_empty() {
        println!(
            "\n  {} {}",
            "untagged".dimmed(),
            format!("({})", untagged.len()).dimmed()
        );
    }

    Ok(())
}

fn list_by_tag(config: &Config, tag: &str) -> Result<()> {
    let skills = skill::discover_all(&config.sources.skills)?;

    let matching: Vec<_> = skills
        .iter()
        .filter(|s| {
            s.frontmatter
                .tags
                .as_ref()
                .map(|t| t.contains(&tag.to_string()))
                .unwrap_or(false)
        })
        .collect();

    println!(
        "{} {}",
        "--- Skills tagged".cyan().bold(),
        tag.cyan().bold()
    );

    if matching.is_empty() {
        println!("\n{}", "No skills found with this tag.".dimmed());
        return Ok(());
    }

    println!();
    for s in &matching {
        let desc = &s.frontmatter.description;
        let truncated: String = desc.chars().take(80).collect();
        let suffix = if desc.len() > 80 { "..." } else { "" };
        println!(
            "  {} {}",
            s.name.green(),
            format!("— {}{}", truncated, suffix).dimmed()
        );
    }

    Ok(())
}

fn list_pipelines(config: &Config) -> Result<()> {
    let skills = skill::discover_all(&config.sources.skills)?;

    // Collect pipeline info
    let mut pipelines: HashMap<String, Vec<(String, String, u32)>> = HashMap::new();
    for s in &skills {
        if let Some(pipeline) = &s.frontmatter.pipeline {
            for (name, stage) in pipeline {
                pipelines.entry(name.clone()).or_default().push((
                    s.name.clone(),
                    stage.stage.clone(),
                    stage.order,
                ));
            }
        }
    }

    if pipelines.is_empty() {
        println!(
            "{}",
            "No pipelines found. Add pipeline metadata to SKILL.md frontmatter.".dimmed()
        );
        return Ok(());
    }

    println!("{}", "--- Pipelines ---".cyan().bold());

    let mut names: Vec<_> = pipelines.keys().collect();
    names.sort();

    for name in names {
        let stages = &pipelines[name];
        let mut sorted = stages.clone();
        sorted.sort_by_key(|s| s.2);

        let stage_summary: Vec<String> = sorted
            .iter()
            .map(|(skill, stage, _)| format!("{} ({})", skill, stage))
            .collect();

        println!(
            "\n  {} {}",
            name.yellow().bold(),
            format!("({} skills)", stages.len()).dimmed()
        );
        println!("  {}", stage_summary.join(" → ").dimmed());
    }

    Ok(())
}

fn list_pipeline(config: &Config, pipeline_name: &str) -> Result<()> {
    let skills = skill::discover_all(&config.sources.skills)?;

    // Collect skills in this pipeline
    let mut stages: Vec<(String, skill::PipelineStage)> = Vec::new();
    let mut all_pipeline_names: HashSet<String> = HashSet::new();

    for s in &skills {
        if let Some(pipeline) = &s.frontmatter.pipeline {
            for name in pipeline.keys() {
                all_pipeline_names.insert(name.clone());
            }
            if let Some(stage) = pipeline.get(pipeline_name) {
                stages.push((s.name.clone(), stage.clone()));
            }
        }
    }

    if stages.is_empty() {
        let mut available: Vec<_> = all_pipeline_names.into_iter().collect();
        available.sort();
        if available.is_empty() {
            anyhow::bail!("No pipelines found in any skill");
        }
        anyhow::bail!(
            "Pipeline '{}' not found. Available: {}",
            pipeline_name,
            available.join(", ")
        );
    }

    // Sort by order
    stages.sort_by_key(|(_, stage)| stage.order);

    println!(
        "{} {}",
        "--- Pipeline:".cyan().bold(),
        pipeline_name.cyan().bold()
    );
    println!();

    let mut last_order = 0;
    for (name, stage) in &stages {
        let after_str = stage
            .after
            .as_ref()
            .map(|a| format!("after: [{}]", a.join(", ")))
            .unwrap_or_default();
        let before_str = stage
            .before
            .as_ref()
            .map(|b| format!("before: [{}]", b.join(", ")))
            .unwrap_or_default();

        let arrows = [after_str, before_str]
            .iter()
            .filter(|s| !s.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join("  ");

        // Show separator between different order levels
        if stage.order != last_order && last_order != 0 {
            println!("    {}", "↓".dimmed());
        }
        last_order = stage.order;

        println!(
            "  {} {} {}  {}",
            format!("{}.", stage.order).dimmed(),
            name.green(),
            format!("({})", stage.stage).yellow(),
            arrows.dimmed()
        );
    }

    Ok(())
}

fn list_missing(config: &Config) -> Result<()> {
    let skills = skill::discover_all(&config.sources.skills)?;
    let skill_map = skill::build_skill_map(skills.clone());
    let known_skills: HashSet<String> = skills.iter().map(|s| s.name.clone()).collect();

    // Extract all cross-references
    let mut all_referenced: HashSet<String> = HashSet::new();
    for skill in &skills {
        let skill_md = skill.path.join("SKILL.md");
        let content = fs::read_to_string(&skill_md)
            .context(format!("Failed to read {}", skill_md.display()))?;
        let refs =
            skill::extract_references_with_filter(&content, &skill.name, Some(&known_skills));
        for r in refs {
            all_referenced.insert(r.target);
        }
    }

    // Find dangling references
    let mut missing: Vec<String> = all_referenced
        .iter()
        .filter(|name| !skill_map.contains_key(*name))
        .cloned()
        .collect();
    missing.sort();

    println!(
        "{}",
        "--- Missing skills (dangling references) ---".cyan().bold()
    );

    if missing.is_empty() {
        println!("{}", "No missing skills found.".green());
    } else {
        println!(
            "{} missing skills referenced:\n",
            missing.len().to_string().red().bold()
        );
        for name in &missing {
            println!("  {} {}", "✗".red(), name.red());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Global, Sources};
    use std::collections::HashMap;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_skills(temp: &TempDir) {
        let skills_dir = temp.path().join("skills");

        let test_skill_dir = skills_dir.join("test-skill");
        fs::create_dir_all(&test_skill_dir).unwrap();
        fs::write(
            test_skill_dir.join("SKILL.md"),
            "---\nname: test-skill\ndescription: Test skill\ntags: [blog, writing]\npipeline:\n  my-pipeline:\n    stage: first\n    order: 1\n    before: [another-skill]\n---\n",
        )
        .unwrap();

        let another_skill_dir = skills_dir.join("another-skill");
        fs::create_dir_all(&another_skill_dir).unwrap();
        fs::write(
            another_skill_dir.join("SKILL.md"),
            "---\nname: another-skill\ndescription: Another test skill\ntags: [blog]\npipeline:\n  my-pipeline:\n    stage: second\n    order: 2\n    after: [test-skill]\n---\n\n<crossrefs>\n  <see ref=\"test-skill\">Related</see>\n</crossrefs>",
        )
        .unwrap();
    }

    #[test]
    fn should_list_default_mode() {
        // Given
        let temp = TempDir::new().unwrap();
        create_test_skills(&temp);

        let config = Config {
            sources: Sources {
                skills: vec![temp.path().join("skills")],
            },
            global: Global {
                targets: vec![],
                skills: vec!["test-skill".to_string()],
            },
            projects: HashMap::new(),
        };

        // When
        let result = list(&config, ListMode::Default);

        // Then
        assert!(result.is_ok());
    }

    #[test]
    fn should_list_refs_for_skill() {
        // Given
        let temp = TempDir::new().unwrap();
        create_test_skills(&temp);

        let config = Config {
            sources: Sources {
                skills: vec![temp.path().join("skills")],
            },
            global: Global {
                targets: vec![],
                skills: vec![],
            },
            projects: HashMap::new(),
        };

        // When
        let result = list(&config, ListMode::Refs("test-skill".to_string()));

        // Then
        assert!(result.is_ok());
    }

    #[test]
    fn should_error_when_skill_not_found_for_refs() {
        // Given
        let temp = TempDir::new().unwrap();
        create_test_skills(&temp);

        let config = Config {
            sources: Sources {
                skills: vec![temp.path().join("skills")],
            },
            global: Global {
                targets: vec![],
                skills: vec![],
            },
            projects: HashMap::new(),
        };

        // When
        let result = list(&config, ListMode::Refs("nonexistent".to_string()));

        // Then
        assert!(result.is_err());
    }

    #[test]
    fn should_list_all_tags_with_counts() {
        // Given
        let temp = TempDir::new().unwrap();
        create_test_skills(&temp);

        let config = Config {
            sources: Sources {
                skills: vec![temp.path().join("skills")],
            },
            global: Global {
                targets: vec![],
                skills: vec![],
            },
            projects: HashMap::new(),
        };

        // When
        let result = list(&config, ListMode::Tags);

        // Then
        assert!(result.is_ok());
    }

    #[test]
    fn should_list_skills_by_tag() {
        // Given
        let temp = TempDir::new().unwrap();
        create_test_skills(&temp);

        let config = Config {
            sources: Sources {
                skills: vec![temp.path().join("skills")],
            },
            global: Global {
                targets: vec![],
                skills: vec![],
            },
            projects: HashMap::new(),
        };

        // When
        let result = list(&config, ListMode::Tag("blog".to_string()));

        // Then
        assert!(result.is_ok());
    }

    #[test]
    fn should_list_all_pipelines() {
        // Given
        let temp = TempDir::new().unwrap();
        create_test_skills(&temp);

        let config = Config {
            sources: Sources {
                skills: vec![temp.path().join("skills")],
            },
            global: Global {
                targets: vec![],
                skills: vec![],
            },
            projects: HashMap::new(),
        };

        // When
        let result = list(&config, ListMode::Pipelines);

        // Then
        assert!(result.is_ok());
    }

    #[test]
    fn should_list_pipeline_in_stage_order() {
        // Given
        let temp = TempDir::new().unwrap();
        create_test_skills(&temp);

        let config = Config {
            sources: Sources {
                skills: vec![temp.path().join("skills")],
            },
            global: Global {
                targets: vec![],
                skills: vec![],
            },
            projects: HashMap::new(),
        };

        // When
        let result = list(&config, ListMode::Pipeline("my-pipeline".to_string()));

        // Then
        assert!(result.is_ok());
    }

    #[test]
    fn should_error_when_pipeline_not_found() {
        // Given
        let temp = TempDir::new().unwrap();
        create_test_skills(&temp);

        let config = Config {
            sources: Sources {
                skills: vec![temp.path().join("skills")],
            },
            global: Global {
                targets: vec![],
                skills: vec![],
            },
            projects: HashMap::new(),
        };

        // When
        let result = list(&config, ListMode::Pipeline("nonexistent".to_string()));

        // Then
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn should_list_missing_skills() {
        // Given
        let temp = TempDir::new().unwrap();
        let skills_dir = temp.path().join("skills");
        let skill_dir = skills_dir.join("referrer");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: referrer\ndescription: Refs nonexistent\n---\n\n<crossrefs>\n  <see ref=\"nonexistent\">Missing</see>\n</crossrefs>",
        )
        .unwrap();

        let config = Config {
            sources: Sources {
                skills: vec![temp.path().join("skills")],
            },
            global: Global {
                targets: vec![],
                skills: vec![],
            },
            projects: HashMap::new(),
        };

        // When
        let result = list(&config, ListMode::Missing);

        // Then
        assert!(result.is_ok());
    }
}
