//! `entwine init`: scaffold the recommended knowledge areas without inventing facts.
use crate::{
    fsafe::{create_new, Written},
    templates,
};
use entwine_core::KnowledgeRole;
use entwine_engine::scan_roles;
use std::{
    collections::BTreeSet,
    io::{self, BufRead, IsTerminal, Write},
    path::Path,
};

/// One recommended area and how it can be created.
struct Area {
    role: KnowledgeRole,
    label: &'static str,
    /// Canonical file under docs/ that would be created.
    file: &'static str,
    /// Directory shown to the user for plural areas.
    directory: Option<&'static str>,
    template: &'static str,
}
const AREAS: [Area; 6] = [
    Area {
        role: KnowledgeRole::Project,
        label: "index.md",
        file: "index.md",
        directory: None,
        template: "",
    },
    Area {
        role: KnowledgeRole::Architecture,
        label: "architecture.md",
        file: "architecture.md",
        directory: None,
        template: templates::ARCHITECTURE,
    },
    Area {
        role: KnowledgeRole::State,
        label: "state.md",
        file: "state.md",
        directory: None,
        template: templates::STATE,
    },
    Area {
        role: KnowledgeRole::Roadmap,
        label: "roadmap.md",
        file: "roadmap.md",
        directory: None,
        template: templates::ROADMAP,
    },
    Area {
        role: KnowledgeRole::Decision,
        label: "decisions/",
        file: "decisions/index.md",
        directory: Some("decisions"),
        template: templates::DECISIONS_INDEX,
    },
    Area {
        role: KnowledgeRole::Spec,
        label: "specs/",
        file: "specs/index.md",
        directory: Some("specs"),
        template: templates::SPECS_INDEX,
    },
];

pub(crate) struct Options {
    pub yes: bool,
    pub dry_run: bool,
}

fn in_git_repository(project: &Path) -> bool {
    project.ancestors().any(|p| p.join(".git").exists())
}

fn confirm(question: &str) -> io::Result<bool> {
    print!("{question} › [Y/n] ");
    io::stdout().flush()?;
    let mut answer = String::new();
    io::stdin().lock().read_line(&mut answer)?;
    Ok(!matches!(
        answer.trim().to_ascii_lowercase().as_str(),
        "n" | "no"
    ))
}

pub(crate) fn run(project: &Path, options: &Options) -> Result<(), Box<dyn std::error::Error>> {
    let docs = project.join("docs");
    if docs
        .symlink_metadata()
        .is_ok_and(|m| m.file_type().is_symlink())
    {
        return Err("docs/ must not be a symbolic link".into());
    }
    println!("Entwine init\n");
    println!(
        "{} Git repository detected",
        if in_git_repository(project) {
            "✓"
        } else {
            "○ No"
        }
    );
    let docs_exists = docs.is_dir();
    println!(
        "{}",
        if docs_exists {
            "✓ docs/ detected"
        } else {
            "○ docs/ not found; it will be created"
        }
    );
    let found = scan_roles(project)?;
    let roles: BTreeSet<_> = found.iter().map(|(_, role)| *role).collect();
    let first_path = |role: KnowledgeRole| {
        found
            .iter()
            .find(|(_, r)| *r == role)
            .map(|(p, _)| format!("docs/{p}"))
    };

    println!("\nProject knowledge\n");
    let mut missing = Vec::new();
    for area in &AREAS {
        let landing = area.directory.is_some() && docs.join(area.file).is_file();
        if roles.contains(&area.role) || landing {
            let at = first_path(area.role).unwrap_or_else(|| format!("docs/{}", area.file));
            println!("✓ {:<16}{at}", area.label);
        } else {
            println!("○ {}", area.label);
            missing.push(area);
        }
    }
    if missing.is_empty() {
        println!("\n✓ already configured\n\nExisting files were not changed.");
        return Ok(());
    }
    if options.dry_run {
        println!("\nWould create:");
        for area in &missing {
            println!("  docs/{}", area.file);
        }
        println!("\nNo files changed.");
        return Ok(());
    }
    if !options.yes {
        if !io::stdin().is_terminal() {
            println!("\nNot an interactive terminal; nothing was changed. Re-run with --yes to create the missing files.");
            return Ok(());
        }
        println!();
        if !confirm("Create missing recommended knowledge files?")? {
            println!("\nNothing was changed.");
            return Ok(());
        }
    }

    // Link the entry point only to documents that exist or are created now.
    let created: BTreeSet<_> = missing.iter().map(|a| a.role).collect();
    let links: Vec<_> = AREAS
        .iter()
        .filter(|a| a.role != KnowledgeRole::Project)
        .filter_map(|a| {
            let path = if created.contains(&a.role) {
                a.file.to_string()
            } else {
                found
                    .iter()
                    .find(|(_, r)| *r == a.role)
                    .map(|(p, _)| p.clone())
                    .or_else(|| docs.join(a.file).is_file().then(|| a.file.to_string()))?
            };
            Some((a.role, path, created.contains(&a.role)))
        })
        .collect();
    let name = project
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Project")
        .to_string();
    println!();
    let mut skipped = false;
    for area in missing {
        let content = if area.role == KnowledgeRole::Project {
            templates::index(&name, &links)
        } else {
            area.template.to_string()
        };
        let relative = format!("docs/{}", area.file);
        match create_new(project, &relative, &content)? {
            Written::Created => println!(
                "✓ Created docs/{}",
                area.directory
                    .map_or_else(|| area.file.to_string(), |d| format!("{d}/"))
            ),
            Written::Exists => {
                skipped = true;
                println!("○ docs/{} already exists; not changed", area.file);
            }
        }
    }
    println!(
        "\nExisting files were not changed.{}",
        if skipped {
            " Some recommended files already exist with another role; see above."
        } else {
            ""
        }
    );
    println!("\nNext: fill in the questions, then run `entwine dev` and `entwine check`.");
    Ok(())
}
