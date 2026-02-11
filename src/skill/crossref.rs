use regex::Regex;
use std::collections::HashSet;

/// A cross-reference to another skill found in SKILL.md body content
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CrossRef {
    /// The name of the referenced skill
    pub target: String,
    /// The line number where the reference was found (1-indexed)
    pub line: usize,
    /// How the reference was detected
    pub method: DetectionMethod,
}

/// Detection method for skill references
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DetectionMethod {
    /// Found in <crossrefs><see ref="..."> XML element
    XmlCrossref,
    /// Backtick-quoted skill name adjacent to contextual words
    BacktickContext,
    /// Mentioned in markdown table under "Related skills" or "Integration" heading
    RelatedTable,
    /// Natural language pattern (e.g., "invoke the X skill", "load X first")
    NaturalLanguage,
}

/// Extract all skill references from SKILL.md body content
///
/// Returns a Vec of CrossRef entries for each detected reference.
/// Filters out self-references (when skill_name matches the reference).
pub fn extract_references(content: &str, skill_name: &str) -> Vec<CrossRef> {
    let mut refs = Vec::new();

    refs.extend(extract_xml_crossrefs(content));
    refs.extend(extract_backtick_context(content));
    refs.extend(extract_related_tables(content));
    refs.extend(extract_natural_language(content));

    // Filter out self-references
    refs.into_iter()
        .filter(|r| r.target != skill_name)
        .collect()
}

/// Build a cross-reference map from skill name to set of referenced skill names
pub fn build_reference_map(
    skills: &[(String, Vec<CrossRef>)],
) -> std::collections::HashMap<String, HashSet<String>> {
    skills
        .iter()
        .map(|(name, refs)| {
            let targets: HashSet<String> = refs.iter().map(|r| r.target.clone()).collect();
            (name.clone(), targets)
        })
        .collect()
}

// --- Detection heuristics ---

fn extract_xml_crossrefs(content: &str) -> Vec<CrossRef> {
    let mut refs = Vec::new();
    let re = Regex::new(r#"<see\s+ref="([a-z0-9]+(?:-[a-z0-9]+)*)">"#).unwrap();

    for (line_num, line) in content.lines().enumerate() {
        for cap in re.captures_iter(line) {
            if let Some(skill_name) = cap.get(1) {
                refs.push(CrossRef {
                    target: skill_name.as_str().to_string(),
                    line: line_num + 1,
                    method: DetectionMethod::XmlCrossref,
                });
            }
        }
    }

    refs
}

fn extract_backtick_context(content: &str) -> Vec<CrossRef> {
    let mut refs = Vec::new();

    // Matches backtick-quoted skill names when adjacent to contextual words
    // Pattern: (skill|invoke|load|use) followed/preceded by `skill-name`
    let re = Regex::new(
        r"(?i)\b(skill|invoke|load|use)\b[^\n`]*`([a-z0-9]+(?:-[a-z0-9]+)*)`|`([a-z0-9]+(?:-[a-z0-9]+)*)`[^\n`]*\b(skill|invoke|load|use)\b"
    ).unwrap();

    for (line_num, line) in content.lines().enumerate() {
        for cap in re.captures_iter(line) {
            // Either group 2 (context before) or group 3 (context after) will match
            let skill_name = cap.get(2).or_else(|| cap.get(3));
            if let Some(name) = skill_name {
                refs.push(CrossRef {
                    target: name.as_str().to_string(),
                    line: line_num + 1,
                    method: DetectionMethod::BacktickContext,
                });
            }
        }
    }

    refs
}

fn extract_related_tables(content: &str) -> Vec<CrossRef> {
    let mut refs = Vec::new();
    let mut in_related_section = false;
    let skill_pattern = Regex::new(r"`([a-z0-9]+(?:-[a-z0-9]+)*)`").unwrap();

    for (line_num, line) in content.lines().enumerate() {
        let line_lower = line.to_lowercase();

        // Detect section headers
        if line_lower.contains("related skill") || line_lower.contains("integration") {
            in_related_section = true;
            continue;
        }

        // Exit section on next header
        if line.starts_with('#') && in_related_section {
            in_related_section = false;
        }

        // Extract skill names from table rows in related sections
        if in_related_section && line.contains('|') {
            for cap in skill_pattern.captures_iter(line) {
                if let Some(name) = cap.get(1) {
                    refs.push(CrossRef {
                        target: name.as_str().to_string(),
                        line: line_num + 1,
                        method: DetectionMethod::RelatedTable,
                    });
                }
            }
        }
    }

    refs
}

fn extract_natural_language(content: &str) -> Vec<CrossRef> {
    let mut refs = Vec::new();

    // Patterns: "invoke the X skill", "load X first", "use X skill", etc.
    // Case-insensitive to handle "Load voice first" and "load voice first"
    let patterns = [
        r"(?i)invoke\s+(?:the\s+)?([a-z0-9]+(?:-[a-z0-9]+)*)\s+skill",
        r"(?i)load\s+([a-z0-9]+(?:-[a-z0-9]+)*)\s+(?:first|skill)",
        r"(?i)use\s+(?:the\s+)?([a-z0-9]+(?:-[a-z0-9]+)*)\s+skill",
        r"(?i)invoke\s+([a-z0-9]+(?:-[a-z0-9]+)*)\s+on",
    ];

    for pattern in &patterns {
        let re = Regex::new(pattern).unwrap();
        for (line_num, line) in content.lines().enumerate() {
            for cap in re.captures_iter(line) {
                if let Some(name) = cap.get(1) {
                    refs.push(CrossRef {
                        target: name.as_str().to_string(),
                        line: line_num + 1,
                        method: DetectionMethod::NaturalLanguage,
                    });
                }
            }
        }
    }

    refs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_extract_xml_crossrefs() {
        // Given
        let content = r#"
  <crossrefs>
    <see ref="dev-workflow">Commit format</see>
    <see ref="bdd">Acceptance criteria</see>
  </crossrefs>
"#;

        // When
        let refs = extract_xml_crossrefs(content);

        // Then
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0].target, "dev-workflow");
        assert_eq!(refs[0].method, DetectionMethod::XmlCrossref);
        assert_eq!(refs[1].target, "bdd");
    }

    #[test]
    fn should_extract_backtick_context_before() {
        // Given
        let content = "invoke `skill-review` on the result";

        // When
        let refs = extract_backtick_context(content);

        // Then
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].target, "skill-review");
        assert_eq!(refs[0].method, DetectionMethod::BacktickContext);
    }

    #[test]
    fn should_extract_backtick_context_after() {
        // Given
        let content = "Use the `voice` skill for tone calibration";

        // When
        let refs = extract_backtick_context(content);

        // Then
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].target, "voice");
        assert_eq!(refs[0].method, DetectionMethod::BacktickContext);
    }

    #[test]
    fn should_extract_from_related_table() {
        // Given
        let content = r#"
## Related skills

| Skill | Purpose |
|-------|---------|
| `skill-craft` | Creating skills |
| `skill-review` | Reviewing quality |
"#;

        // When
        let refs = extract_related_tables(content);

        // Then
        assert_eq!(refs.len(), 2);
        assert!(refs.iter().any(|r| r.target == "skill-craft"));
        assert!(refs.iter().any(|r| r.target == "skill-review"));
    }

    #[test]
    fn should_extract_natural_language_invoke_the() {
        // Given
        let content = "You should invoke the skill-review skill to verify quality";

        // When
        let refs = extract_natural_language(content);

        // Then
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].target, "skill-review");
        assert_eq!(refs[0].method, DetectionMethod::NaturalLanguage);
    }

    #[test]
    fn should_extract_natural_language_load_first() {
        // Given
        let content = "Load voice first before editing articles";

        // When
        let refs = extract_natural_language(content);

        // Then
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].target, "voice");
    }

    #[test]
    fn should_filter_self_references() {
        // Given
        let content = r#"
  <crossrefs>
    <see ref="skill-craft">This skill</see>
    <see ref="other-skill">Another skill</see>
  </crossrefs>
"#;

        // When
        let refs = extract_references(content, "skill-craft");

        // Then
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].target, "other-skill");
    }

    #[test]
    fn should_build_reference_map() {
        // Given
        let skills = vec![
            (
                "skill-a".to_string(),
                vec![
                    CrossRef {
                        target: "skill-b".to_string(),
                        line: 1,
                        method: DetectionMethod::XmlCrossref,
                    },
                    CrossRef {
                        target: "skill-c".to_string(),
                        line: 2,
                        method: DetectionMethod::XmlCrossref,
                    },
                ],
            ),
            ("skill-b".to_string(), vec![]),
        ];

        // When
        let map = build_reference_map(&skills);

        // Then
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("skill-a").unwrap().len(), 2);
        assert!(map.get("skill-a").unwrap().contains("skill-b"));
        assert!(map.get("skill-a").unwrap().contains("skill-c"));
        assert_eq!(map.get("skill-b").unwrap().len(), 0);
    }

    #[test]
    fn should_record_line_numbers() {
        // Given
        let content = "Line 1\nLine 2 with invoke `my-skill` here\nLine 3";

        // When
        let refs = extract_backtick_context(content);

        // Then
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].line, 2);
    }
}
