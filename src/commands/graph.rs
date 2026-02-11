use anyhow::Result;
use std::collections::HashMap;
use std::fs;

use crate::config::Config;
use crate::graph::SkillGraph;
use crate::skill;

#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Dot,
    Text,
    Json,
    Mermaid,
}

impl OutputFormat {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "dot" => Some(Self::Dot),
            "text" => Some(Self::Text),
            "json" => Some(Self::Json),
            "mermaid" => Some(Self::Mermaid),
            _ => None,
        }
    }
}

pub fn graph(config: &Config, format: OutputFormat) -> Result<()> {
    // Discover all skills
    let all_skills = skill::discover_all(&config.sources.skills)?;

    // Extract cross-references
    let mut crossrefs = HashMap::new();
    for skill in &all_skills {
        let skill_md = skill.path.join("SKILL.md");
        let content = fs::read_to_string(&skill_md)?;
        let refs = skill::extract_references(&content, &skill.name);
        if !refs.is_empty() {
            crossrefs.insert(skill.name.clone(), refs);
        }
    }

    // Build the graph
    let skill_graph = SkillGraph::from_crossrefs(&crossrefs);

    // Output in requested format
    let output = match format {
        OutputFormat::Dot => skill_graph.to_dot(),
        OutputFormat::Text => skill_graph.to_text(),
        OutputFormat::Json => skill_graph.to_json(),
        OutputFormat::Mermaid => skill_graph.to_mermaid(),
    };

    println!("{}", output);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_parse_output_format_case_insensitive() {
        // Given/When/Then
        assert!(matches!(
            OutputFormat::from_str("dot"),
            Some(OutputFormat::Dot)
        ));
        assert!(matches!(
            OutputFormat::from_str("DOT"),
            Some(OutputFormat::Dot)
        ));
        assert!(matches!(
            OutputFormat::from_str("text"),
            Some(OutputFormat::Text)
        ));
        assert!(matches!(
            OutputFormat::from_str("json"),
            Some(OutputFormat::Json)
        ));
        assert!(matches!(
            OutputFormat::from_str("mermaid"),
            Some(OutputFormat::Mermaid)
        ));
        assert!(OutputFormat::from_str("invalid").is_none());
    }
}
