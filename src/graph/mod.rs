//! Dependency graph construction and analysis (requires `graph` feature)

use petgraph::algo::tarjan_scc;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::{HashMap, HashSet};

use crate::skill::CrossRef;

/// A skill dependency graph with analysis results
#[derive(Debug)]
pub struct SkillGraph {
    /// The underlying directed graph
    graph: DiGraph<String, ()>,

    /// Map from skill name to node index
    name_to_node: HashMap<String, NodeIndex>,

    /// Detected clusters (strongly connected components)
    pub clusters: Vec<Vec<String>>,

    /// Root skills (no incoming edges)
    pub roots: Vec<String>,

    /// Leaf skills (no outgoing edges)
    pub leaves: Vec<String>,

    /// Bridge skills (articulation points)
    pub bridges: Vec<String>,
}

impl SkillGraph {
    /// Build a skill graph from cross-reference data
    pub fn from_crossrefs(crossrefs: &HashMap<String, Vec<CrossRef>>) -> Self {
        let mut graph = DiGraph::new();
        let mut name_to_node = HashMap::new();

        // Collect all unique skill names
        let mut all_skills: HashSet<String> = HashSet::new();
        for (source, refs) in crossrefs {
            all_skills.insert(source.clone());
            for r in refs {
                all_skills.insert(r.target.clone());
            }
        }

        // Add all skills as nodes
        for skill in &all_skills {
            let node = graph.add_node(skill.clone());
            name_to_node.insert(skill.clone(), node);
        }

        // Add edges from cross-references
        for (source, refs) in crossrefs {
            let source_node = name_to_node[source];
            for r in refs {
                if let Some(&target_node) = name_to_node.get(&r.target) {
                    graph.add_edge(source_node, target_node, ());
                }
            }
        }

        // Analyze the graph
        let clusters = detect_clusters(&graph, &name_to_node);
        let roots = find_roots(&graph, &name_to_node);
        let leaves = find_leaves(&graph, &name_to_node);
        let bridges = find_bridges(&graph, &name_to_node);

        SkillGraph {
            graph,
            name_to_node,
            clusters,
            roots,
            leaves,
            bridges,
        }
    }

    /// Export graph as Graphviz DOT format
    pub fn to_dot(&self) -> String {
        let mut output = String::from("digraph SkillGraph {\n");
        output.push_str("  rankdir=LR;\n");
        output.push_str("  node [shape=box, style=rounded];\n\n");

        // Add nodes
        for (name, _) in &self.name_to_node {
            let color = if self.roots.contains(name) {
                "lightblue"
            } else if self.leaves.contains(name) {
                "lightgreen"
            } else if self.bridges.contains(name) {
                "orange"
            } else {
                "white"
            };
            output.push_str(&format!(
                "  \"{}\" [fillcolor={}, style=\"rounded,filled\"];\n",
                name, color
            ));
        }

        output.push('\n');

        // Add edges
        for edge in self.graph.edge_references() {
            let source = &self.graph[edge.source()];
            let target = &self.graph[edge.target()];
            output.push_str(&format!("  \"{}\" -> \"{}\";\n", source, target));
        }

        output.push_str("}\n");
        output
    }

    /// Export graph as human-readable adjacency list
    pub fn to_text(&self) -> String {
        let mut output = String::new();

        output.push_str("# Skill Dependency Graph\n\n");

        // Show analysis summary
        output.push_str(&format!("Skills: {}\n", self.name_to_node.len()));
        output.push_str(&format!("Clusters: {}\n", self.clusters.len()));
        output.push_str(&format!("Roots: {}\n", self.roots.len()));
        output.push_str(&format!("Leaves: {}\n", self.leaves.len()));
        output.push_str(&format!("Bridges: {}\n\n", self.bridges.len()));

        // Show adjacency list
        output.push_str("## Dependencies\n\n");
        let mut sorted_skills: Vec<_> = self.name_to_node.keys().collect();
        sorted_skills.sort();

        for skill in sorted_skills {
            let node = self.name_to_node[skill];
            let targets: Vec<String> = self
                .graph
                .edges(node)
                .map(|e| self.graph[e.target()].clone())
                .collect();

            if targets.is_empty() {
                output.push_str(&format!("{}: (none)\n", skill));
            } else {
                output.push_str(&format!("{}: {}\n", skill, targets.join(", ")));
            }
        }

        output
    }

    /// Export graph as JSON
    pub fn to_json(&self) -> String {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        for (name, &idx) in &self.name_to_node {
            nodes.push(serde_json::json!({
                "id": name,
                "is_root": self.roots.contains(name),
                "is_leaf": self.leaves.contains(name),
                "is_bridge": self.bridges.contains(name),
            }));

            for edge in self.graph.edges(idx) {
                let target = &self.graph[edge.target()];
                edges.push(serde_json::json!({
                    "source": name,
                    "target": target,
                }));
            }
        }

        serde_json::json!({
            "nodes": nodes,
            "edges": edges,
            "clusters": self.clusters,
        })
        .to_string()
    }

    /// Export graph as Mermaid diagram
    pub fn to_mermaid(&self) -> String {
        let mut output = String::from("graph LR\n");

        for edge in self.graph.edge_references() {
            let source = &self.graph[edge.source()];
            let target = &self.graph[edge.target()];
            output.push_str(&format!(
                "  {}[{}] --> {}[{}]\n",
                sanitize_mermaid(source),
                source,
                sanitize_mermaid(target),
                target
            ));
        }

        output
    }
}

fn sanitize_mermaid(s: &str) -> String {
    s.replace('-', "_")
}

fn detect_clusters(
    graph: &DiGraph<String, ()>,
    _name_to_node: &HashMap<String, NodeIndex>,
) -> Vec<Vec<String>> {
    // Use Tarjan's algorithm to find strongly connected components
    let sccs = tarjan_scc(graph);

    let mut clusters = Vec::new();
    for scc in sccs {
        let cluster: Vec<String> = scc.iter().map(|&idx| graph[idx].clone()).collect();

        // Only include clusters with more than one skill
        if cluster.len() > 1 {
            clusters.push(cluster);
        }
    }

    clusters
}

fn find_roots(
    graph: &DiGraph<String, ()>,
    name_to_node: &HashMap<String, NodeIndex>,
) -> Vec<String> {
    let mut roots = Vec::new();

    for (name, &idx) in name_to_node {
        // Root skills have no incoming edges
        if graph
            .edges_directed(idx, petgraph::Direction::Incoming)
            .count()
            == 0
        {
            roots.push(name.clone());
        }
    }

    roots.sort();
    roots
}

fn find_leaves(
    graph: &DiGraph<String, ()>,
    name_to_node: &HashMap<String, NodeIndex>,
) -> Vec<String> {
    let mut leaves = Vec::new();

    for (name, &idx) in name_to_node {
        // Leaf skills have no outgoing edges
        if graph
            .edges_directed(idx, petgraph::Direction::Outgoing)
            .count()
            == 0
        {
            leaves.push(name.clone());
        }
    }

    leaves.sort();
    leaves
}

fn find_bridges(
    graph: &DiGraph<String, ()>,
    name_to_node: &HashMap<String, NodeIndex>,
) -> Vec<String> {
    // Articulation points - nodes whose removal would increase connected components
    // For directed graphs, this is approximate - we look for nodes that are the only path
    // between different parts of the graph

    let mut bridges = Vec::new();

    // Simple heuristic: a node is a bridge if it has both incoming and outgoing edges
    // and removing it would disconnect some nodes
    for (name, &idx) in name_to_node {
        let incoming = graph
            .edges_directed(idx, petgraph::Direction::Incoming)
            .count();
        let outgoing = graph
            .edges_directed(idx, petgraph::Direction::Outgoing)
            .count();

        // Bridge candidates have both incoming and outgoing edges
        if incoming > 0 && outgoing > 0 {
            bridges.push(name.clone());
        }
    }

    bridges.sort();
    bridges
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::{CrossRef, DetectionMethod};

    fn test_crossref(target: &str) -> CrossRef {
        CrossRef {
            target: target.to_string(),
            line: 1,
            method: DetectionMethod::XmlCrossref,
        }
    }

    #[test]
    fn should_build_graph_from_crossrefs() {
        // Given: skill-a → skill-b → skill-c
        let mut crossrefs = HashMap::new();
        crossrefs.insert("skill-a".to_string(), vec![test_crossref("skill-b")]);
        crossrefs.insert("skill-b".to_string(), vec![test_crossref("skill-c")]);

        // When
        let graph = SkillGraph::from_crossrefs(&crossrefs);

        // Then
        assert_eq!(graph.name_to_node.len(), 3);
    }

    #[test]
    fn should_identify_root_skills() {
        // Given: skill-a → skill-b (skill-a is root)
        let mut crossrefs = HashMap::new();
        crossrefs.insert("skill-a".to_string(), vec![test_crossref("skill-b")]);

        // When
        let graph = SkillGraph::from_crossrefs(&crossrefs);

        // Then
        assert_eq!(graph.roots.len(), 1);
        assert!(graph.roots.contains(&"skill-a".to_string()));
    }

    #[test]
    fn should_identify_leaf_skills() {
        // Given: skill-a → skill-b (skill-b is leaf)
        let mut crossrefs = HashMap::new();
        crossrefs.insert("skill-a".to_string(), vec![test_crossref("skill-b")]);

        // When
        let graph = SkillGraph::from_crossrefs(&crossrefs);

        // Then
        assert_eq!(graph.leaves.len(), 1);
        assert!(graph.leaves.contains(&"skill-b".to_string()));
    }

    #[test]
    fn should_detect_clusters() {
        // Given: skill-a ↔ skill-b (circular reference, forms a cluster)
        let mut crossrefs = HashMap::new();
        crossrefs.insert("skill-a".to_string(), vec![test_crossref("skill-b")]);
        crossrefs.insert("skill-b".to_string(), vec![test_crossref("skill-a")]);

        // When
        let graph = SkillGraph::from_crossrefs(&crossrefs);

        // Then
        assert_eq!(graph.clusters.len(), 1);
        assert_eq!(graph.clusters[0].len(), 2);
    }

    #[test]
    fn should_generate_dot_output() {
        // Given
        let mut crossrefs = HashMap::new();
        crossrefs.insert("skill-a".to_string(), vec![test_crossref("skill-b")]);

        // When
        let graph = SkillGraph::from_crossrefs(&crossrefs);
        let dot = graph.to_dot();

        // Then
        assert!(dot.contains("digraph SkillGraph"));
        assert!(dot.contains("\"skill-a\" -> \"skill-b\""));
    }

    #[test]
    fn should_generate_json_output() {
        // Given
        let mut crossrefs = HashMap::new();
        crossrefs.insert("skill-a".to_string(), vec![test_crossref("skill-b")]);

        // When
        let graph = SkillGraph::from_crossrefs(&crossrefs);
        let json = graph.to_json();

        // Then
        assert!(json.contains("\"nodes\""));
        assert!(json.contains("\"edges\""));
        assert!(json.contains("skill-a"));
    }

    #[test]
    fn should_generate_mermaid_output() {
        // Given
        let mut crossrefs = HashMap::new();
        crossrefs.insert("skill-a".to_string(), vec![test_crossref("skill-b")]);

        // When
        let graph = SkillGraph::from_crossrefs(&crossrefs);
        let mermaid = graph.to_mermaid();

        // Then
        assert!(mermaid.contains("graph LR"));
        assert!(mermaid.contains("skill_a"));
        assert!(mermaid.contains("-->"));
    }
}
