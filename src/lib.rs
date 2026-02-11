//! Loadout: Skill lifecycle management for AI agents
//!
//! This library provides modules for managing SKILL.md files across
//! multiple source directories and linking them into tool discovery paths.

pub mod commands;
pub mod config;
#[cfg(feature = "graph")]
pub mod graph;
pub mod linker;
pub mod skill;
