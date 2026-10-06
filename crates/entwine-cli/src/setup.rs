//! `entwine setup`: scaffold repository-native validation and publishing.
use crate::{
    ci::{self, Layout},
    fsafe::{create_new, Written},
    provider::{self, Provider, Repository},
};
use std::{
    fs,
    path::{Component, Path},
};

pub(crate) struct Options {
    pub dry_run: bool,
    pub provider: Option<ProviderChoice>,
}
#[derive(Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum ProviderChoice {
    Github,
    Gitlab,
    Bitbucket,
}

#[derive(Debug, PartialEq, Eq)]
enum Change {
    Create {
        path: String,
        content: String,
    },
    /// Prepend to an existing shared CI file that was verified safe to extend.
    Prepend {
        path: String,
        content: String,
    },
    Already {
        path: String,
    },
    /// Left untouched; the user must integrate manually.
    Manual {
        path: String,
        reason: String,
    },
}

#[derive(Debug)]
struct Plan {
    provider: Provider,
    changes: Vec<Change>,
    triggers: Vec<String>,
    next_steps: Vec<String>,
}

fn is_entwine_file(path: &Path) -> Option<bool> {
    let meta = fs::symlink_metadata(path).ok()?;
    if !meta.is_file() {
        return Some(false);
    }
    Some(fs::read_to_string(path).is_ok_and(|text| text.contains(ci::MARKER)))
}

/// Derive the repository-relative project directory, rejecting names that are unsafe to
/// embed in YAML or shell.
fn layout(root: &Path, project: &Path) -> Result<Layout, String> {
    let relative = project
        .strip_prefix(root)
        .map_err(|_| "The project is outside the Git repository".to_string())?;
    let mut prefix = String::new();
    for part in relative.components() {
        let Component::Normal(name) = part else {
            return Err("Unsupported project path".into());
        };
        let name = name.to_str().ok_or("The project path must be UTF-8")?;
        if name.is_empty()
            || name.starts_with('-')
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
        {
            return Err(format!(
                "Project directory name {name:?} contains characters that cannot be safely embedded in CI configuration. Use letters, digits, '.', '_' and '-'."
            ));
        }
        prefix.push_str(name);
        prefix.push('/');
    }
    Ok(Layout { prefix })
}

fn plan(
    provider: Provider,
    root: &Path,
    layout: &Layout,
    default_branch: &str,
    version: &str,
) -> Plan {
    let file_change = |path: &str, content: String| -> Change {
        match is_entwine_file(&root.join(path)) {
            None => Change::Create {
                path: path.into(),
                content,
            },
            Some(true) => Change::Already { path: path.into() },
            Some(false) => Change::Manual {
                path: path.into(),
                reason: "exists and was not created by Entwine; it was not changed".into(),
            },
        }
    };
    match provider {
        Provider::GitHub => Plan {
            provider,
            changes: vec![file_change(ci::GITHUB_PATH, ci::github(version, layout))],
            triggers: vec![
                "pull requests changing repository files  → entwine check".into(),
                "pushes to the default branch  → check, build, GitHub Pages".into(),
            ],
            next_steps: vec![
                "In the repository, open Settings → Pages and set Source to \"GitHub Actions\"."
                    .into(),
                "Pages availability for private repositories depends on your GitHub plan.".into(),
            ],
        },
        Provider::GitLab => {
            let mut changes = vec![file_change(ci::GITLAB_PATH, ci::gitlab(version, layout))];
            let root_file = root.join(ci::GITLAB_ROOT);
            changes.push(match fs::symlink_metadata(&root_file) {
                Err(_) => Change::Create { path: ci::GITLAB_ROOT.into(), content: ci::gitlab_include() },
                Ok(meta) if !meta.is_file() => Change::Manual {
                    path: ci::GITLAB_ROOT.into(),
                    reason: "is not a regular file".into(),
                },
                Ok(_) => {
                    let text = fs::read_to_string(&root_file).unwrap_or_default();
                    let top_level = |key: &str| text.lines().any(|l| l.starts_with(key));
                    if text.contains(ci::GITLAB_PATH) {
                        Change::Already { path: ci::GITLAB_ROOT.into() }
                    } else if text.starts_with("---")
                        || top_level("include:")
                        || top_level("stages:")
                        || top_level("workflow:")
                    {
                        Change::Manual {
                            path: ci::GITLAB_ROOT.into(),
                            reason: format!(
                                "already defines include, stages, or workflow; add this to it yourself:\n      include:\n        - local: {}",
                                ci::GITLAB_PATH
                            ),
                        }
                    } else {
                        Change::Prepend { path: ci::GITLAB_ROOT.into(), content: ci::gitlab_include() }
                    }
                }
            });
            Plan {
                provider,
                changes,
                triggers: vec![
                    "merge requests changing repository files  → entwine check".into(),
                    "default branch pipelines  → check, build, GitLab Pages".into(),
                ],
                next_steps: vec![
                    "GitLab Pages must be enabled for the project or instance (requires GitLab 17.10 or later).".into(),
                    "The site URL appears under Deploy → Pages after the first default-branch pipeline.".into(),
                    "On GitLab Self-Managed, set the CI/CD variable ENTWINE_IMAGE if the runner cannot pull node:22.".into(),
                ],
            }
        }
        Provider::Bitbucket => {
            let content = ci::bitbucket(version, layout, default_branch);
            // Bitbucket has no include mechanism: a shared pipelines file is never edited.
            let changes = match is_entwine_file(&root.join(ci::BITBUCKET_PATH)) {
                None => vec![Change::Create {
                    path: ci::BITBUCKET_PATH.into(),
                    content,
                }],
                Some(true) => vec![Change::Already {
                    path: ci::BITBUCKET_PATH.into(),
                }],
                Some(false) => {
                    let merge = Change::Manual {
                        path: ci::BITBUCKET_PATH.into(),
                        reason: format!(
                            "exists; not changed. Merge the pipelines from {} into it",
                            ci::BITBUCKET_SNIPPET
                        ),
                    };
                    match is_entwine_file(&root.join(ci::BITBUCKET_SNIPPET)) {
                        None => vec![
                            Change::Create {
                                path: ci::BITBUCKET_SNIPPET.into(),
                                content,
                            },
                            merge,
                        ],
                        Some(true) => vec![
                            Change::Already {
                                path: ci::BITBUCKET_SNIPPET.into(),
                            },
                            merge,
                        ],
                        Some(false) => vec![Change::Manual {
                            path: ci::BITBUCKET_SNIPPET.into(),
                            reason: "exists and was not created by Entwine; it was not changed"
                                .into(),
                        }],
                    }
                }
            };
            Plan {
                provider,
                changes,
                triggers: vec![
                    "pull requests changing repository files  → entwine check".into(),
                    format!("pushes to {default_branch}  → check, build, publish to <workspace>.bitbucket.io/<repository>/"),
                ],
                next_steps: vec![
                    "Bitbucket static hosting serves one site per workspace, from a repository named <workspace>.bitbucket.io (main branch).".into(),
                    "1. Create the repository <workspace>.bitbucket.io (skip if it exists).".into(),
                    "2. Create a repository access token on it with write access to repositories.".into(),
                    "3. In this repository: Repository settings → Pipelines → Repository variables, add ENTWINE_SITE_TOKEN (secured) with that token.".into(),
                    "4. Enable Pipelines for this repository.".into(),
                    "The site is public even if the repository is private, and Bitbucket caches pages for about 15 minutes.".into(),
                    "Entwine never stores or prints the token; it exists only as a Bitbucket secured variable.".into(),
                ],
            }
        }
        Provider::Unknown => Plan {
            provider,
            changes: Vec::new(),
            triggers: Vec::new(),
            next_steps: Vec::new(),
        },
    }
}

fn apply(root: &Path, change: &Change) -> Result<String, Box<dyn std::error::Error>> {
    Ok(match change {
        Change::Create { path, content } => match create_new(root, path, content)? {
            Written::Created => format!("✓ Created {path}"),
            Written::Exists => format!("✓ {path} already exists; not changed"),
        },
        Change::Prepend { path, content } => {
            let target = root.join(path);
            let existing = fs::read_to_string(&target)?;
            let meta = fs::symlink_metadata(&target)?;
            if meta.file_type().is_symlink() || !meta.is_file() {
                return Err(format!("{path} is not a regular file").into());
            }
            fs::write(&target, format!("{content}\n{existing}"))?;
            format!("✓ Updated {path} (added an include; the rest is unchanged)")
        }
        Change::Already { path } => format!("✓ {path} already configured"),
        Change::Manual { path, reason } => format!("⚠ {path}: {reason}"),
    })
}

fn describe(change: &Change, dry_run: bool) -> String {
    match change {
        Change::Create { path, .. } => format!(
            "{} {path}",
            if dry_run { "Would create:" } else { "Create:" }
        ),
        Change::Prepend { path, .. } => {
            format!("Would update: {path} (prepend an include; the rest is unchanged)")
        }
        Change::Already { path } => format!("Already configured: {path}"),
        Change::Manual { path, reason } => format!("Manual step: {path}: {reason}"),
    }
}

pub(crate) fn run(project: &Path, options: &Options) -> Result<(), Box<dyn std::error::Error>> {
    let version = env!("CARGO_PKG_VERSION");
    let repository: Repository = provider::inspect(project);
    println!("Entwine setup\n");
    let Some(root) = repository.root.clone() else {
        println!("⚠ No Git repository detected.\n");
        fallback();
        return Ok(());
    };
    let root = root.canonicalize()?;
    println!("✓ Git repository detected");
    if !project.join("docs").is_dir() {
        println!("⚠ No docs/ directory yet. Run `entwine init` to scaffold project knowledge.");
    }
    let provider = match options.provider {
        Some(ProviderChoice::Github) => Provider::GitHub,
        Some(ProviderChoice::Gitlab) => Provider::GitLab,
        Some(ProviderChoice::Bitbucket) => Provider::Bitbucket,
        None => repository.provider,
    };
    match (&repository.remote_name, &repository.remote) {
        (Some(name), Some(remote)) => println!(
            "✓ Remote {name}: {}/{}{}",
            remote.host,
            remote.path,
            if options.provider.is_some() {
                " (provider chosen with --provider)"
            } else {
                ""
            }
        ),
        _ => println!("○ No usable Git remote found"),
    }
    if provider == Provider::Unknown {
        println!("⚠ No supported native publishing provider detected.");
        if repository.remote.is_some() {
            println!("  Self-hosted GitLab, GitHub, or Bitbucket? Pass --provider gitlab|github|bitbucket.");
        }
        println!();
        fallback();
        return Ok(());
    }
    println!("✓ Provider: {}\n", provider.name());
    let layout = layout(&root, project).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    let branch = repository.default_branch.as_deref().unwrap_or("main");
    let plan = plan(provider, &root, &layout, branch, version);

    if options.dry_run {
        println!("Provider: {}", plan.provider.name());
        for change in &plan.changes {
            println!("{}", describe(change, true));
        }
        print_triggers(&plan);
        println!("\nNo files changed.");
        return Ok(());
    }
    let mut all_configured = true;
    for change in &plan.changes {
        if !matches!(change, Change::Already { .. }) {
            all_configured = false;
        }
        println!("{}", apply(&root, change)?);
    }
    if all_configured {
        println!("\n✓ already configured");
        return Ok(());
    }
    print_triggers(&plan);
    println!("\nNext (one-time):");
    for step in &plan.next_steps {
        println!("  {step}");
    }
    println!("\nGenerated pipelines pin @entwine/cli@{version}. Commit the generated files to enable validation.");
    println!(
        "To remove this setup, delete the generated files. `entwine build` still produces dist/."
    );
    Ok(())
}

fn print_triggers(plan: &Plan) {
    if !plan.triggers.is_empty() {
        println!("\nTriggers:");
        for trigger in &plan.triggers {
            println!("  {trigger}");
        }
    }
}

fn fallback() {
    println!("Entwine output remains portable:\n\n    entwine build\n\nPublish dist/ using your existing static hosting pipeline.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_paths_are_validated_before_entering_ci() {
        let root = Path::new("/repo");
        assert_eq!(layout(root, Path::new("/repo")).unwrap().prefix, "");
        assert_eq!(
            layout(root, Path::new("/repo/apps/site")).unwrap().prefix,
            "apps/site/"
        );
        for bad in [
            "/repo/a b",
            "/repo/a;rm",
            "/repo/$(x)",
            "/repo/-flag",
            "/repo/a\"b",
        ] {
            assert!(layout(root, Path::new(bad)).is_err(), "{bad}");
        }
        assert!(layout(root, Path::new("/elsewhere")).is_err());
    }
}
