//! Scan and parse each document once, resolve a canonical model, then project it.
mod graph_layout;
mod markdown;
mod projections;
mod render;
mod resolve;

use entwine_core::*;
pub use projections::{context_json, context_markdown};
pub use render::{render_graph, render_site, StaticFile};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::Path,
};
use walkdir::WalkDir;

/// Compilation owns canonical knowledge and its three deterministic projections.
#[derive(Debug)]
pub struct Compilation {
    pub knowledge: KnowledgeBase,
    pub site: SiteModel,
    pub graph: GraphModel,
    pub context: ContextModel,
    pub diagnostics: Vec<Diagnostic>,
    /// Validated static assets, stored relative to docs/.
    pub assets: Vec<StaticFile>,
}
impl Compilation {
    /// Fatal diagnostics prevent output publication.
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == DiagnosticSeverity::Error)
    }
}

pub(crate) fn error(source: &str, line: Option<usize>, message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        severity: DiagnosticSeverity::Error,
        source: source.into(),
        line,
        message: message.into(),
    }
}

/// Compile `project/docs` without configuration or network access.
pub fn compile(project: &Path) -> io::Result<Compilation> {
    let root = project.join("docs");
    if !root.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "No docs/ directory at {}. Create docs/index.md to start.",
                project.display()
            ),
        ));
    }
    if fs::symlink_metadata(&root)?.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "docs/ must not be a symbolic link",
        ));
    }
    let mut diagnostics = Vec::new();
    let mut paths = Vec::new();
    let mut assets = Vec::new();
    for entry in WalkDir::new(&root).follow_links(false) {
        let entry = entry.map_err(io::Error::other)?;
        if entry.path() == root {
            continue;
        }
        let relative = entry.path().strip_prefix(&root).map_err(io::Error::other)?;
        let Some(source) = relative.to_str().map(|s| s.replace('\\', "/")) else {
            diagnostics.push(error("docs/", None, "Documentation paths must be UTF-8"));
            continue;
        };
        if entry.file_type().is_symlink() {
            diagnostics.push(error(
                &source,
                None,
                "Symbolic links are not supported inside docs/; use regular files",
            ));
            continue;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        if source.ends_with(".md") {
            paths.push((source, entry.into_path()));
        } else {
            if source.split('/').any(|p| {
                p.is_empty()
                    || p == "."
                    || p == ".."
                    || p == "__entwine"
                    || p.chars().any(|c| c.is_control() || ":?#%".contains(c))
            }) {
                diagnostics.push(error(&source, None, "Invalid or reserved asset path"));
                continue;
            }
            assets.push(StaticFile {
                path: source,
                contents: fs::read(entry.path())?,
            });
        }
    }
    paths.sort_by(|a, b| a.0.cmp(&b.0));
    assets.sort_by(|a, b| a.path.cmp(&b.path));
    if paths.is_empty() {
        diagnostics.push(error(
            "docs/",
            None,
            "No Markdown documents found. Create docs/index.md.",
        ));
    }
    let mut parsed = Vec::new();
    let mut routes = BTreeMap::new();
    for (source, path) in paths {
        let route = match Route::from_source(&source) {
            Ok(route) => route,
            Err(message) => {
                diagnostics.push(error(&source, None, message));
                continue;
            }
        };
        if let Some(previous) = routes.insert(route.clone(), source.clone()) {
            diagnostics.push(error(
                &source,
                None,
                format!(
                    "Route collision at {}: also produced by {previous}",
                    route.as_str()
                ),
            ));
        }
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::InvalidData => {
                diagnostics.push(error(&source, None, "Markdown must contain valid UTF-8"));
                continue;
            }
            Err(e) => return Err(e),
        };
        parsed.push(markdown::parse(&source, route, &text, &mut diagnostics));
    }
    let targets: BTreeMap<_, _> = parsed
        .iter()
        .map(|p| {
            (
                p.document.id.0.clone(),
                (
                    p.document.id.clone(),
                    p.document.route.clone(),
                    p.document.headings.clone(),
                ),
            )
        })
        .collect();
    let asset_names = assets
        .iter()
        .map(|a| a.path.clone())
        .collect::<BTreeSet<_>>();
    for document in &mut parsed {
        resolve::links(document, &targets, &asset_names, &mut diagnostics);
    }
    let documents: Vec<_> = parsed.into_iter().map(markdown::finish).collect();
    let relations = documents
        .iter()
        .flat_map(|d| {
            d.links.iter().filter_map(|l| {
                l.target.as_ref().map(|t| Relation {
                    source: d.id.clone(),
                    target: t.clone(),
                    kind: RelationKind::References,
                })
            })
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let knowledge = KnowledgeBase {
        documents,
        relations,
    };
    for document in &knowledge.documents {
        if document.route.as_str() != "/" && knowledge.backlinks(&document.id).is_empty() {
            diagnostics.push(Diagnostic {
                severity: DiagnosticSeverity::Warning,
                source: document.id.0.clone(),
                line: None,
                message: "No incoming document relationships".into(),
            });
        }
    }
    let (site, graph, context) = projections::project(&knowledge);
    // Detect files that would occupy directories or overwrite generated pages.
    let mut output_paths: Vec<String> = site
        .pages
        .iter()
        .map(|p| format!("{}index.html", p.route.as_str().trim_start_matches('/')))
        .collect();
    output_paths.extend([
        "__entwine/style.css".into(),
        "__entwine/graph/index.html".into(),
    ]);
    for (index, path) in output_paths.iter().enumerate() {
        if output_paths.iter().skip(index + 1).any(|other| {
            other.starts_with(&(path.clone() + "/")) || path.starts_with(&(other.clone() + "/"))
        }) {
            diagnostics.push(error(
                "docs/",
                None,
                format!("Generated file/directory collision involving {path}"),
            ));
        }
    }
    for asset in &assets {
        if output_paths.iter().any(|p| {
            p == &asset.path
                || p.starts_with(&(asset.path.clone() + "/"))
                || asset.path.starts_with(&(p.clone() + "/"))
        }) {
            diagnostics.push(error(
                &asset.path,
                None,
                "Asset collides with generated documentation output",
            ));
        }
    }
    diagnostics
        .sort_by(|a, b| (&a.source, a.line, &a.message).cmp(&(&b.source, b.line, &b.message)));
    Ok(Compilation {
        knowledge,
        site,
        graph,
        context,
        diagnostics,
        assets,
    })
}
