//! CLI presentation, safe output publication, and local development lifecycle.
mod ci;
mod fsafe;
mod init;
mod knowledge;
mod output;
mod provider;
mod server;
mod setup;
mod templates;
use clap::{Parser, Subcommand};
use entwine_core::{ArtifactKind, DiagnosticSeverity};
use entwine_engine::{
    compile_with_config, context_json, context_markdown, load_config, Compilation,
};
use std::{
    io::{self, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

#[derive(Parser)]
#[command(name = "entwine", version, about = "Project knowledge, connected.")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    /// Compile and serve docs/, rebuilding on local changes.
    Dev {
        /// Project directory containing docs/.
        #[arg(default_value = ".")]
        project: PathBuf,
        /// Loopback port to serve on (0 picks a free port).
        #[arg(long, default_value_t = 4173)]
        port: u16,
    },
    /// Generate deployable HTML/CSS documentation and graph in dist/.
    Build {
        /// Project directory containing docs/.
        #[arg(default_value = ".")]
        project: PathBuf,
    },
    /// Generate the documentation and its viewable static SVG graph.
    Graph {
        /// Project directory containing docs/.
        #[arg(default_value = ".")]
        project: PathBuf,
    },
    /// Validate documentation and report recommended knowledge coverage.
    Check {
        /// Project directory containing docs/.
        #[arg(default_value = ".")]
        project: PathBuf,
        /// Fail when a recommended knowledge area is missing.
        #[arg(long)]
        strict_knowledge: bool,
    },
    /// Scaffold the recommended project knowledge files without overwriting anything.
    Init {
        /// Project directory containing docs/.
        #[arg(default_value = ".")]
        project: PathBuf,
        /// Create missing files without prompting.
        #[arg(long, short)]
        yes: bool,
        /// Show what would be created.
        #[arg(long)]
        dry_run: bool,
    },
    /// Configure repository-native validation and publishing (GitHub, GitLab, Bitbucket).
    Setup {
        /// Project directory containing docs/.
        #[arg(default_value = ".")]
        project: PathBuf,
        /// Show the plan without changing any file.
        #[arg(long)]
        dry_run: bool,
        /// Override provider detection, for example for self-hosted GitLab.
        #[arg(long, value_enum)]
        provider: Option<setup::ProviderChoice>,
    },
    /// Write deterministic project context to stdout.
    Context {
        /// Project directory containing docs/.
        #[arg(default_value = ".")]
        project: PathBuf,
        /// Emit versioned structured JSON instead of Markdown.
        #[arg(long)]
        json: bool,
    },
}

fn diagnostics(compilation: &Compilation) {
    for diagnostic in &compilation.diagnostics {
        let severity = match diagnostic.severity {
            DiagnosticSeverity::Error => "error",
            DiagnosticSeverity::Warning => "warning",
        };
        let line = diagnostic.line.map_or(String::new(), |l| format!(":{l}"));
        let source = if let Some(path) = diagnostic.source.strip_prefix("repo:") {
            path.to_string()
        } else if diagnostic.source == "entwine.toml" {
            diagnostic.source.clone()
        } else {
            format!("docs/{}", diagnostic.source.trim_start_matches("docs/"))
        };
        eprintln!("{severity}: {source}{line}: {}", diagnostic.message);
    }
}
fn compile_project(project: &Path) -> io::Result<Compilation> {
    let repository = provider::inspect(project);
    let source = provider::source(&repository);
    let config = load_config(project)?;
    compile_with_config(
        project,
        repository.root.as_deref().unwrap_or(project),
        source.as_ref(),
        &config,
    )
}
fn checked(project: &Path) -> Result<Compilation, Box<dyn std::error::Error>> {
    let compilation = compile_project(project)?;
    diagnostics(&compilation);
    if compilation.has_errors() {
        return Err("Documentation validation failed; output was not changed".into());
    }
    Ok(compilation)
}
fn summary(compilation: &Compilation) {
    let documents = || {
        compilation
            .knowledge
            .documents
            .iter()
            .filter(|d| d.artifact == ArtifactKind::Documentation)
    };
    eprintln!(
        "Entwine\n\n✓ {} documents\n✓ {} internal links\n✓ {} relationships\n✓ {} repository references",
        documents().count(),
        documents()
            .flat_map(|d| &d.links)
            .filter(|l| l.target.is_some())
            .count(),
        compilation.knowledge.relations.len(),
        compilation
            .knowledge
            .documents
            .iter()
            .map(|d| d.repository_references.len())
            .sum::<usize>()
    );
    knowledge::agent_summary(compilation);
}
/// Resolve the project argument, naming the path when it cannot be used.
fn resolve_project(project: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let resolved = project.canonicalize().map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            format!("Project directory {} does not exist", project.display())
        } else {
            format!(
                "Cannot read project directory {}: {error}",
                project.display()
            )
        }
    })?;
    if !resolved.is_dir() {
        return Err(format!(
            "Project path {} is not a directory; pass the repository root, not a file",
            project.display()
        )
        .into());
    }
    Ok(resolved)
}
/// Remind users to keep generated output out of commits when Git would track it.
fn ignore_hint(project: &Path) {
    let status = std::process::Command::new("git")
        .args(["check-ignore", "-q", "dist/index.html"])
        .current_dir(project)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    if status.is_ok_and(|s| s.code() == Some(1)) {
        eprintln!("\nTip: dist/ is not ignored by Git. Add `dist/` to .gitignore to keep generated output out of commits.");
    }
}
fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Command::Dev { project, port } => server::dev(&resolve_project(&project)?, port),
        Command::Build { project } | Command::Graph { project } => {
            let project = resolve_project(&project)?;
            let compilation = checked(&project)?;
            output::publish(&project, &compilation)?;
            summary(&compilation);
            eprintln!(
                "✓ static documentation generated\n\n{}\nGraph: {}",
                project.join("dist").display(),
                project.join("dist/__entwine/graph/index.html").display()
            );
            ignore_hint(&project);
            Ok(())
        }
        Command::Check {
            project,
            strict_knowledge,
        } => {
            let compilation = checked(&resolve_project(&project)?)?;
            summary(&compilation);
            let missing = knowledge::report(&compilation, strict_knowledge);
            if strict_knowledge && !missing.is_empty() {
                return Err(format!(
                    "{} recommended knowledge area{} missing (--strict-knowledge)",
                    missing.len(),
                    if missing.len() == 1 { " is" } else { "s are" }
                )
                .into());
            }
            eprintln!("\n✓ validation passed");
            Ok(())
        }
        Command::Init {
            project,
            yes,
            dry_run,
        } => init::run(&resolve_project(&project)?, &init::Options { yes, dry_run }),
        Command::Setup {
            project,
            dry_run,
            provider,
        } => setup::run(
            &resolve_project(&project)?,
            &setup::Options { dry_run, provider },
        ),
        Command::Context { project, json } => {
            let compilation = checked(&resolve_project(&project)?)?;
            let content = if json {
                context_json(&compilation.context)?
            } else {
                context_markdown(&compilation.context)
            };
            writeln!(io::stdout().lock(), "{content}")?;
            Ok(())
        }
    }
}
fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
