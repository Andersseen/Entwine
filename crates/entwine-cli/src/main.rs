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
use entwine_core::DiagnosticSeverity;
use entwine_engine::{compile_with_source, context_json, context_markdown, Compilation};
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
        #[arg(default_value = ".")]
        project: PathBuf,
        #[arg(long, default_value_t = 4173)]
        port: u16,
    },
    /// Generate deployable HTML/CSS documentation and graph in dist/.
    Build {
        #[arg(default_value = ".")]
        project: PathBuf,
    },
    /// Generate the documentation and its viewable static SVG graph.
    Graph {
        #[arg(default_value = ".")]
        project: PathBuf,
    },
    /// Validate documentation and report recommended knowledge coverage.
    Check {
        #[arg(default_value = ".")]
        project: PathBuf,
        /// Fail when a recommended knowledge area is missing.
        #[arg(long)]
        strict_knowledge: bool,
    },
    /// Scaffold the recommended project knowledge files without overwriting anything.
    Init {
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
        #[arg(default_value = ".")]
        project: PathBuf,
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
        eprintln!(
            "{severity}: docs/{}{line}: {}",
            diagnostic.source.trim_start_matches("docs/"),
            diagnostic.message
        );
    }
}
fn compile_project(project: &Path) -> io::Result<Compilation> {
    let repository = provider::inspect(project);
    let source = provider::source(&repository);
    compile_with_source(
        project,
        repository.root.as_deref().unwrap_or(project),
        source.as_ref(),
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
    eprintln!(
        "Entwine\n\n✓ {} documents\n✓ {} internal links\n✓ {} relationships\n✓ {} repository references",
        compilation.knowledge.documents.len(),
        compilation
            .knowledge
            .documents
            .iter()
            .flat_map(|d| &d.links)
            .filter(|l| l.target.is_some())
            .count(),
        compilation.knowledge.relations.len(),
        compilation.knowledge.documents.iter().map(|d| d.repository_references.len()).sum::<usize>()
    );
}
fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Command::Dev { project, port } => server::dev(&project.canonicalize()?, port),
        Command::Build { project } | Command::Graph { project } => {
            let project = project.canonicalize()?;
            let compilation = checked(&project)?;
            output::publish(&project, &compilation)?;
            summary(&compilation);
            eprintln!(
                "✓ static documentation generated\n\n{}\nGraph: {}",
                project.join("dist").display(),
                project.join("dist/__entwine/graph/index.html").display()
            );
            Ok(())
        }
        Command::Check {
            project,
            strict_knowledge,
        } => {
            let compilation = checked(&project.canonicalize()?)?;
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
        } => init::run(&project.canonicalize()?, &init::Options { yes, dry_run }),
        Command::Setup {
            project,
            dry_run,
            provider,
        } => setup::run(
            &project.canonicalize()?,
            &setup::Options { dry_run, provider },
        ),
        Command::Context { project, json } => {
            let compilation = checked(&project.canonicalize()?)?;
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
