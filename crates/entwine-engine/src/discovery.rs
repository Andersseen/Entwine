//! Opt-in discovery of agent-facing repository knowledge. Only reads files;
//! nothing found is executed, copied, or published by this module.
use crate::warning;
use entwine_core::*;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use std::{
    cell::RefCell,
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};
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

/// Everything discovery found, plus how many convention files `.gitignore` kept out.
pub(crate) struct Discovered {
    pub found: Vec<Found>,
    pub ignored: usize,
}

/// Git ignore rules, evaluated without Git: the `.gitignore` files from the repository root down
/// to each directory, plus the repository's `.git/info/exclude`. The Git index, global
/// ignore files, and the `git` program are never consulted, so results depend only on files
/// inside the repository and are identical on every machine. A repository without `.git`
/// still honors the `.gitignore` files it contains.
struct IgnoreRules {
    repository: PathBuf,
    cache: RefCell<HashMap<PathBuf, Option<Gitignore>>>,
}
impl IgnoreRules {
    fn new(repository: &Path) -> Self {
        Self {
            repository: repository.to_path_buf(),
            cache: RefCell::default(),
        }
    }
    fn rules_in(&self, directory: &Path) -> Option<Gitignore> {
        let mut builder = GitignoreBuilder::new(directory);
        let mut any = builder.add(directory.join(".gitignore")).is_none()
            && directory.join(".gitignore").is_file();
        if directory == self.repository {
            let exclude = directory.join(".git").join("info").join("exclude");
            if exclude.is_file() {
                any |= builder.add(exclude).is_none();
            }
        }
        if !any {
            return None;
        }
        builder.build().ok()
    }
    /// Whether `path` (below the repository) is ignored. The caller prunes ignored directories,
    /// so ignored parents never need re-checking here.
    fn ignored(&self, path: &Path, is_dir: bool) -> bool {
        let Ok(relative) = path.strip_prefix(&self.repository) else {
            return false;
        };
        let mut directory = self.repository.clone();
        let mut verdict = false;
        let mut components = relative.components().peekable();
        loop {
            let matched = {
                let mut cache = self.cache.borrow_mut();
                let rules = cache
                    .entry(directory.clone())
                    .or_insert_with(|| self.rules_in(&directory));
                rules.as_ref().map(|r| {
                    let m = r.matched(path, is_dir);
                    (m.is_ignore(), m.is_whitelist())
                })
            };
            match matched {
                Some((true, _)) => verdict = true,
                Some((_, true)) => verdict = false,
                _ => {}
            }
            let Some(next) = components.next() else { break };
            if components.peek().is_none() {
                break; // `next` is the entry itself, not a directory to descend into
            }
            directory.push(next);
        }
        verdict
    }
}

/// Walk `project` (never `docs/`, never through symbolic links) for enabled conventions.
/// `prefix` is the project's repository-relative directory (empty at the repository root).
pub(crate) fn discover(
    project: &Path,
    repository: &Path,
    prefix: &str,
    config: &Config,
    diagnostics: &mut Vec<Diagnostic>,
) -> Discovered {
    let mut found = Vec::new();
    let mut ignored = 0;
    if !config.discovers_agent_knowledge() {
        return Discovered { found, ignored };
    }
    let rules = IgnoreRules::new(repository);
    let walker = WalkDir::new(project)
        .follow_links(false)
        .max_depth(MAX_DEPTH)
        .into_iter()
        .filter_entry(|entry| {
            entry.depth() == 0
                || !(entry.file_type().is_dir()
                    && (skipped(&entry.file_name().to_string_lossy())
                        || (entry.depth() == 1 && entry.file_name() == "docs")
                        || rules.ignored(entry.path(), true)))
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
        if rules.ignored(entry.path(), false) {
            ignored += 1;
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
                .map(|directory| skill_resources(directory, &rules))
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
    Discovered { found, ignored }
}

/// File names colocated with a skill manifest; contents are never read. Ignored files are not
/// listed, so a private scratch file next to a skill stays invisible.
fn skill_resources(directory: &Path, rules: &IgnoreRules) -> Vec<String> {
    let mut resources: Vec<String> = WalkDir::new(directory)
        .follow_links(false)
        .max_depth(4)
        .into_iter()
        .filter_entry(|e| {
            e.depth() == 0
                || !(skipped(&e.file_name().to_string_lossy())
                    || rules.ignored(e.path(), e.file_type().is_dir()))
        })
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
