//! CLI presentation, safe output publication, and local development lifecycle.
mod output;
mod server;
use clap::{Parser, Subcommand};
use entwine_core::DiagnosticSeverity;
use entwine_engine::{compile, context_json, context_markdown, Compilation};
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
    /// Validate documentation without writing output.
    Check {
        #[arg(default_value = ".")]
        project: PathBuf,
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
            diagnostic.source, diagnostic.message
        );
    }
}
fn checked(project: &Path) -> Result<Compilation, Box<dyn std::error::Error>> {
    let compilation = compile(project)?;
    diagnostics(&compilation);
    if compilation.has_errors() {
        return Err("Documentation validation failed; output was not changed".into());
    }
    Ok(compilation)
}
fn summary(compilation: &Compilation) {
    eprintln!(
        "Entwine\n\n✓ {} documents\n✓ {} internal links\n✓ {} relationships",
        compilation.knowledge.documents.len(),
        compilation
            .knowledge
            .documents
            .iter()
            .flat_map(|d| &d.links)
            .filter(|l| l.target.is_some())
            .count(),
        compilation.knowledge.relations.len()
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
        Command::Check { project } => {
            let compilation = checked(&project.canonicalize()?)?;
            summary(&compilation);
            eprintln!("✓ validation passed");
            Ok(())
        }
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
