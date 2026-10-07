//! Opt-in discovery of agent-facing repository knowledge. Only reads files;
//! nothing found is executed, copied, or published by this module.
use crate::warning;
use entwine_core::*;
use std::{fs, path::Path};
use walkdir::WalkDir;

const MAX_DEPTH: usize = 12;
const MAX_VISITED: usize = 100_000;
const MAX_FILE_BYTES: u64 = 1_000_000;
const MAX_RESOURCES: usize = 200;
const SKIPPED_DIRECTORIES: [&str; 11] = [
    ".git",
    "node_modules",
    "target",
    "dist",
    "vendor",
    ".venv",
    "__pycache__",
    ".next",
    ".turbo",
    ".cache",
    ".svn",
];

/// One discovered file before it is parsed.
pub(crate) struct Found {
    pub convention: AgentConvention,
    /// Repository-relative path.
    pub path: String,
    pub text: String,
    pub resources: Vec<String>,
}

fn skipped(name: &str) -> bool {
    SKIPPED_DIRECTORIES.contains(&name) || name.starts_with(".entwine-")
}

/// Walk `project` (never `docs/`, never through symbolic links) for enabled conventions.
/// `prefix` is the project's repository-relative directory (empty at the repository root).
pub(crate) fn discover(
    project: &Path,
    prefix: &str,
    config: &Config,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<Found> {
    let mut found = Vec::new();
    if !config.discovers_agent_knowledge() {
        return found;
    }
    let walker = WalkDir::new(project)
        .follow_links(false)
        .max_depth(MAX_DEPTH)
        .into_iter()
        .filter_entry(|entry| {
            entry.depth() == 0
                || !(entry.file_type().is_dir()
                    && (skipped(&entry.file_name().to_string_lossy())
                        || (entry.depth() == 1 && entry.file_name() == "docs")))
        });
    for (visited, entry) in walker.flatten().enumerate() {
        if visited >= MAX_VISITED {
            diagnostics.push(warning(
                "entwine.toml",
                None,
                "Agent knowledge discovery stopped after 100000 entries",
            ));
            break;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        let Some(convention) = entry
            .file_name()
            .to_str()
            .and_then(AgentConvention::from_file_name)
        else {
            continue;
        };
        let enabled = match convention.kind() {
            ArtifactKind::Skill => config.discovery.skills,
            _ => config.discovery.agent_instructions,
        };
        if !enabled {
            continue;
        }
        let Ok(relative) = entry.path().strip_prefix(project) else {
            continue;
        };
        let Some(relative) = relative.to_str().map(|s| s.replace('\\', "/")) else {
            continue;
        };
        let path = join(prefix, &relative);
        if entry.metadata().is_ok_and(|m| m.len() > MAX_FILE_BYTES) {
            diagnostics.push(warning(
                &format!("{REPOSITORY_ID_PREFIX}{path}"),
                None,
                "Skipped: file is larger than 1 MB",
            ));
            continue;
        }
        let text = match fs::read_to_string(entry.path()) {
            Ok(text) => text,
            Err(_) => {
                diagnostics.push(warning(
                    &format!("{REPOSITORY_ID_PREFIX}{path}"),
                    None,
                    "Skipped: file is not readable UTF-8",
                ));
                continue;
            }
        };
        let resources = if convention == AgentConvention::SkillMd {
            entry
                .path()
                .parent()
                .map(skill_resources)
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        found.push(Found {
            convention,
            path,
            text,
            resources,
        });
    }
    found.sort_by(|a, b| a.path.cmp(&b.path));
    found
}

/// File names colocated with a skill manifest; contents are never read.
fn skill_resources(directory: &Path) -> Vec<String> {
    let mut resources: Vec<String> = WalkDir::new(directory)
        .follow_links(false)
        .max_depth(4)
        .into_iter()
        .filter_entry(|e| e.depth() == 0 || !skipped(&e.file_name().to_string_lossy()))
        .flatten()
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| {
            e.path()
                .strip_prefix(directory)
                .ok()
                .and_then(|p| p.to_str())
                .map(|s| s.replace('\\', "/"))
        })
        .filter(|name| name != "SKILL.md")
        .collect();
    resources.sort();
    resources.truncate(MAX_RESOURCES);
    resources
}

pub(crate) fn join(prefix: &str, relative: &str) -> String {
    if prefix.is_empty() {
        relative.into()
    } else {
        format!("{prefix}/{relative}")
    }
}

pub(crate) fn parent(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(parent, _)| parent)
}

fn segment(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "-_.".contains(c) {
                c
            } else {
                '-'
            }
        })
        .collect();
    match cleaned.strip_prefix('.') {
        Some(rest) => format!("dot-{rest}"),
        None => cleaned,
    }
}

/// Generated, collision-checked route for a discovered artifact.
pub(crate) fn route_for(convention: AgentConvention, path: &str) -> Result<Route, String> {
    let directory = parent(path);
    let base = if convention == AgentConvention::SkillMd {
        let name = if directory.is_empty() {
            "root".to_string()
        } else {
            directory
                .split('/')
                .map(segment)
                .collect::<Vec<_>>()
                .join("/")
        };
        format!("__entwine/agents/skills/{name}")
    } else {
        let stem = convention.file_name().trim_end_matches(".md");
        let dir = directory.split('/').filter(|p| !p.is_empty()).map(segment);
        let parts: Vec<_> = dir.chain(std::iter::once(stem.to_string())).collect();
        format!("__entwine/agents/instructions/{}", parts.join("/"))
    };
    Route::generated(&base)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_are_safe_and_never_start_with_a_dot() {
        let route = route_for(AgentConvention::SkillMd, ".claude/skills/triage/SKILL.md").unwrap();
        assert_eq!(
            route.as_str(),
            "/__entwine/agents/skills/dot-claude/skills/triage/"
        );
        let route = route_for(AgentConvention::AgentsMd, "packages/web app/AGENTS.md").unwrap();
        assert_eq!(
            route.as_str(),
            "/__entwine/agents/instructions/packages/web-app/AGENTS/"
        );
        let root = route_for(AgentConvention::SkillMd, "SKILL.md").unwrap();
        assert_eq!(root.as_str(), "/__entwine/agents/skills/root/");
    }
}
