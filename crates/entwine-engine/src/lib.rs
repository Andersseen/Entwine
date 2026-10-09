//! Scan and parse each document once, resolve a canonical model, then project it.
mod agents_page;
mod discovery;
mod flowview_renderer;
mod graph_layout;
mod graph_page;
mod markdown;
mod projections;
mod render;
mod resolve;

pub use agents_page::render_agents;
use entwine_core::*;
pub use graph_page::{render_graph, render_graph_script};
pub use projections::{context_json, context_markdown};
pub use render::{render_knowledge, render_site, StaticFile};
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
    /// Validated repository paths used by links, including missing file targets.
    pub repository_dependencies: Vec<String>,
    /// The configuration this compilation used (defaults when no `entwine.toml` exists).
    pub config: Config,
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

pub(crate) fn warning(source: &str, line: Option<usize>, message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        severity: DiagnosticSeverity::Warning,
        source: source.into(),
        line,
        message: message.into(),
    }
}

/// Record a route and return an earlier source whose route differs only by case.
fn case_collision(
    seen: &mut BTreeMap<String, (String, String)>,
    route: &Route,
    source: &str,
) -> Option<String> {
    let (previous_route, previous) = seen
        .entry(route.as_str().to_lowercase())
        .or_insert_with(|| (route.as_str().to_owned(), source.to_owned()))
        .clone();
    (previous_route != route.as_str()).then_some(previous)
}

/// Read the optional `entwine.toml` beside `docs/`. A missing file is the zero-config default;
/// a malformed file or unknown key is an error naming the problem.
pub fn load_config(project: &Path) -> io::Result<Config> {
    let path = project.join("entwine.toml");
    match fs::symlink_metadata(&path) {
        Ok(meta) if meta.file_type().is_symlink() => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "entwine.toml must not be a symbolic link",
        )),
        Ok(_) => {
            let text = fs::read_to_string(&path)
                .map_err(|e| io::Error::new(e.kind(), format!("Cannot read entwine.toml: {e}")))?;
            parse_config(&text)
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(e),
    }
}

/// Parse `entwine.toml` text. Unknown keys are rejected so typos never silently publish or hide.
pub fn parse_config(text: &str) -> io::Result<Config> {
    toml::from_str(text).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Invalid entwine.toml: {}", e.message().trim()),
        )
    })
}

/// Every file of a complete static site using the selected page renderer and
/// Entwine's existing Graph, Knowledge, Agents, and asset renderers.
pub fn render(compilation: &Compilation) -> std::io::Result<Vec<StaticFile>> {
    render_selected(compilation)
}

/// Complete static site rendered through the built-in page renderer, for
/// parity comparisons and rollback.
pub fn render_builtin(compilation: &Compilation) -> Vec<StaticFile> {
    render_views(compilation, render_site(&compilation.site))
}

/// Render the configured page shell and the existing Entwine-owned views.
/// Internal template failures are returned as renderer errors before publication.
pub fn render_selected(compilation: &Compilation) -> std::io::Result<Vec<StaticFile>> {
    let pages = render_site_with_renderer(&compilation.site, compilation.config.site.renderer)?;
    Ok(render_views(compilation, pages))
}

/// Render documentation pages with an explicit renderer, useful for controlled
/// comparisons while the Flowview option remains experimental.
pub fn render_site_with_renderer(
    site: &SiteModel,
    renderer: RendererKind,
) -> std::io::Result<Vec<StaticFile>> {
    match renderer {
        RendererKind::Builtin => Ok(render_site(site)),
        RendererKind::Flowview => flowview_renderer::render_site(site),
    }
}

fn render_views(compilation: &Compilation, mut files: Vec<StaticFile>) -> Vec<StaticFile> {
    files.push(render_graph(&compilation.graph));
    files.push(render_graph_script());
    files.push(render_knowledge(&compilation.site));
    files.extend(render_agents(&compilation.site));
    files.extend(compilation.assets.clone());
    files
}

/// Find which knowledge roles existing Markdown plays, without validating or rendering it.
/// Unreadable or non-UTF-8 files are skipped; symbolic links are never followed.
pub fn scan_roles(project: &Path) -> io::Result<Vec<(String, KnowledgeRole)>> {
    let root = project.join("docs");
    let mut found = Vec::new();
    if !root.is_dir() || fs::symlink_metadata(&root)?.file_type().is_symlink() {
        return Ok(found);
    }
    for entry in WalkDir::new(&root)
        .follow_links(false)
        .into_iter()
        .flatten()
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let Some(source) = entry
            .path()
            .strip_prefix(&root)
            .ok()
            .and_then(|p| p.to_str())
            .map(|s| s.replace('\\', "/"))
            .filter(|s| s.ends_with(".md"))
        else {
            continue;
        };
        if let Ok(text) = fs::read_to_string(entry.path()) {
            let role = if source == "README.md"
                && !root.join("index.md").is_file()
                && markdown::role_of(&source, &text) == KnowledgeRole::Other
            {
                KnowledgeRole::Project
            } else {
                markdown::role_of(&source, &text)
            };
            found.push((source, role));
        }
    }
    found.sort();
    Ok(found)
}

/// Compile `project/docs` without configuration or network access.
pub fn compile(project: &Path) -> io::Result<Compilation> {
    compile_with_source(project, project, None)
}

/// Compile with the project's `entwine.toml` (or defaults) and no source metadata.
pub fn compile_configured(project: &Path) -> io::Result<Compilation> {
    compile_with_config(project, project, None, &load_config(project)?)
}

/// Compile with a validated repository boundary and optional local source metadata.
/// Git inspection belongs to callers; compilation never invokes Git or the network.
pub fn compile_with_source(
    project: &Path,
    repository: &Path,
    source: Option<&RepositorySource>,
) -> io::Result<Compilation> {
    compile_with_config(project, repository, source, &Config::default())
}

/// Compile with explicit configuration. Discovery of agent-facing artifacts only happens
/// when the configuration enables it; publication is a separate, explicit setting.
pub fn compile_with_config(
    project: &Path,
    repository: &Path,
    source: Option<&RepositorySource>,
    config: &Config,
) -> io::Result<Compilation> {
    if source.is_some_and(|s| {
        !s.file_base_url.starts_with("https://")
            || !s.file_base_url.ends_with('/')
            || s.file_base_url
                .chars()
                .any(|c| c.is_control() || c.is_whitespace() || "@?#\\".contains(c))
    }) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Repository source must be a credential-free HTTPS file URL prefix",
        ));
    }
    let repository = repository.canonicalize()?;
    let project = project.canonicalize()?;
    if !project.starts_with(&repository) {
        return Err(io::Error::other(
            "Project is outside the repository boundary",
        ));
    }
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
    let mut folded_routes = BTreeMap::new();
    let has_index = paths.iter().any(|(s, _)| s == "index.md");
    for (source, path) in paths {
        let route = match Route::from_source(&source) {
            Ok(_) if source == "README.md" && !has_index => Route::home(),
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
        // Case-insensitive hosts (macOS, Windows) would silently merge these outputs.
        if let Some(previous) = case_collision(&mut folded_routes, &route, &source) {
            diagnostics.push(error(
                &source,
                None,
                format!("Routes differ only by case: {previous} and {source}; case-insensitive file systems cannot publish both"),
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
        let mut document = markdown::parse(&source, route, &text, &mut diagnostics);
        if source == "README.md"
            && !has_index
            && document
                .document
                .metadata
                .kind
                .as_deref()
                .and_then(KnowledgeRole::from_type)
                .is_none()
        {
            document.document.role = KnowledgeRole::Project;
        }
        parsed.push(document);
    }
    let targets: BTreeMap<_, _> = parsed
        .iter()
        .map(|p| {
            (
                p.document.id.0.clone(),
                (
                    p.document.id.clone(),
                    p.document.route.clone(),
                    p.document
                        .headings
                        .iter()
                        .map(|h| h.id.clone())
                        .chain(p.document.anchors.clone())
                        .collect::<Vec<_>>(),
                ),
            )
        })
        .collect();
    let asset_names = assets
        .iter()
        .map(|a| a.path.clone())
        .collect::<BTreeSet<_>>();
    let mut repository_dependencies = Vec::new();
    let docs_prefix = root
        .strip_prefix(&repository)
        .unwrap_or(&root)
        .to_string_lossy()
        .replace('\\', "/");
    let project_prefix = project
        .strip_prefix(&repository)
        .unwrap_or(Path::new(""))
        .to_string_lossy()
        .replace('\\', "/");
    let publish_agents = config.site.include_agent_knowledge;
    let mut agent_parsed = Vec::new();
    let mut artifact_paths: resolve::Artifacts = BTreeMap::new();
    for found in discovery::discover(&project, &project_prefix, config, &mut diagnostics) {
        let id = format!("{REPOSITORY_ID_PREFIX}{}", found.path);
        let route = match discovery::route_for(found.convention, &found.path) {
            Ok(route) => route,
            Err(message) => {
                diagnostics.push(warning(&id, None, message));
                continue;
            }
        };
        if let Some(previous) = routes.insert(route.clone(), found.path.clone()) {
            diagnostics.push(error(
                &id,
                None,
                format!(
                    "Route collision at {}: also produced by {previous}",
                    route.as_str()
                ),
            ));
            continue;
        }
        if let Some(previous) = case_collision(&mut folded_routes, &route, &found.path) {
            diagnostics.push(error(
                &id,
                None,
                format!("Routes differ only by case: {previous} and {}", found.path),
            ));
        }
        let mut local = Vec::new();
        let mut document = markdown::parse(&id, route.clone(), &found.text, &mut local);
        // Agent files are written for tools: tolerate their frontmatter and links.
        diagnostics.extend(local.into_iter().map(|mut d| {
            d.severity = DiagnosticSeverity::Warning;
            d.source = id.clone();
            d
        }));
        let (name, description) = if found.convention == AgentConvention::SkillMd {
            markdown::skill_fields(&found.text)
        } else {
            (None, None)
        };
        // Titles and scopes are relative to the project, not to a larger repository.
        let relative = |path: &str| {
            path.strip_prefix(&project_prefix)
                .map_or(path, |rest| rest.trim_start_matches('/'))
                .to_string()
        };
        let directory = relative(discovery::parent(&found.path));
        let doc = &mut document.document;
        doc.role = KnowledgeRole::Other;
        doc.artifact = found.convention.kind();
        doc.title = if found.convention == AgentConvention::SkillMd {
            name.clone()
                .or_else(|| {
                    doc.headings
                        .iter()
                        .find(|h| h.level == 1)
                        .map(|h| h.text.clone())
                })
                .unwrap_or_else(|| {
                    humanize(
                        directory
                            .rsplit('/')
                            .next()
                            .filter(|d| !d.is_empty())
                            .unwrap_or("skill"),
                    )
                })
        } else {
            relative(&found.path)
        };
        doc.agent = Some(AgentDetails {
            convention: found.convention,
            path: found.path.clone(),
            scope: (found.convention != AgentConvention::SkillMd).then(|| directory.clone()),
            name,
            description,
            skill_directory: (found.convention == AgentConvention::SkillMd)
                .then(|| discovery::parent(&found.path).to_string()),
            resources: found.resources.clone(),
        });
        artifact_paths.insert(found.path.clone(), (doc.id.clone(), route));
        agent_parsed.push(document);
    }
    for document in &mut parsed {
        resolve::links(
            document,
            &targets,
            &asset_names,
            &root,
            &repository,
            source,
            &artifact_paths,
            publish_agents,
            &mut repository_dependencies,
            &mut diagnostics,
        );
    }
    let doc_paths: BTreeMap<String, (DocumentId, Route)> = parsed
        .iter()
        .map(|p| {
            (
                format!("{docs_prefix}/{}", p.document.id.0),
                (p.document.id.clone(), p.document.route.clone()),
            )
        })
        .collect();
    for document in &mut agent_parsed {
        let path = document
            .document
            .agent
            .as_ref()
            .map(|a| a.path.clone())
            .unwrap_or_default();
        resolve::artifact_links(
            document,
            &path,
            &resolve::ArtifactContext {
                documents: &doc_paths,
                artifacts: &artifact_paths,
                repository: &repository,
            },
            &mut repository_dependencies,
            &mut diagnostics,
        );
    }
    let mut documents: Vec<_> = parsed.into_iter().map(markdown::finish).collect();
    documents.extend(agent_parsed.into_iter().map(markdown::finish));
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
    for document in knowledge
        .documents
        .iter()
        .filter(|d| d.artifact == ArtifactKind::Documentation)
    {
        if document.route.as_str() != "/" && knowledge.backlinks(&document.id).is_empty() {
            diagnostics.push(Diagnostic {
                severity: DiagnosticSeverity::Warning,
                source: document.id.0.clone(),
                line: None,
                message: "No incoming document relationships".into(),
            });
        }
    }
    let (mut site, graph, context) = projections::project(&knowledge, publish_agents, &docs_prefix);
    projections::source_links(&mut site, &knowledge, &root, &repository, source);
    let mut page_routes = BTreeMap::new();
    for page in &site.pages {
        if let Some(previous) = case_collision(&mut page_routes, &page.route, page.route.as_str()) {
            diagnostics.push(error(
                "docs/",
                None,
                format!(
                    "Generated routes differ only by case: {previous} and {}",
                    page.route.as_str()
                ),
            ));
        }
    }
    // Detect files that would occupy directories or overwrite generated pages.
    let mut output_paths: Vec<String> = site
        .pages
        .iter()
        .map(|p| format!("{}index.html", p.route.as_str().trim_start_matches('/')))
        .collect();
    output_paths.extend([
        "__entwine/style.css".into(),
        "__entwine/graph.js".into(),
        "__entwine/graph/index.html".into(),
        "__entwine/knowledge/index.html".into(),
    ]);
    if let Some(agents) = &site.agents {
        output_paths.extend(
            [
                "__entwine/agents/index.html",
                "__entwine/agents/instructions/index.html",
                "__entwine/agents/skills/index.html",
                "__entwine/agents/scopes/index.html",
            ]
            .map(String::from),
        );
        output_paths.extend(
            agents
                .entries
                .iter()
                .map(|e| format!("{}index.html", e.route.as_str().trim_start_matches('/'))),
        );
    }
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
    let mut portable_paths = BTreeMap::new();
    for path in output_paths.iter().chain(assets.iter().map(|a| &a.path)) {
        if let Some(previous) = portable_paths.insert(path.to_lowercase(), path.clone()) {
            if previous != *path {
                diagnostics.push(error(
                    "docs/",
                    None,
                    format!("Output paths differ only by case: {previous} and {path}"),
                ));
            }
        }
    }
    let portable: Vec<_> = portable_paths.keys().collect();
    for pair in portable.windows(2) {
        if pair[1].starts_with(&(pair[0].to_string() + "/")) {
            diagnostics.push(error(
                "docs/",
                None,
                format!(
                    "Portable file/directory collision: {} and {}",
                    pair[0], pair[1]
                ),
            ));
        }
    }
    diagnostics
        .sort_by(|a, b| (&a.source, a.line, &a.message).cmp(&(&b.source, b.line, &b.message)));
    repository_dependencies.sort();
    repository_dependencies.dedup();
    Ok(Compilation {
        knowledge,
        site,
        graph,
        context,
        diagnostics,
        assets,
        repository_dependencies,
        config: config.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routes_differing_only_by_case_collide() {
        let mut seen = BTreeMap::new();
        let upper = Route::from_source("Guide.md").unwrap();
        let lower = Route::from_source("guide.md").unwrap();
        assert_eq!(case_collision(&mut seen, &upper, "Guide.md"), None);
        assert_eq!(case_collision(&mut seen, &upper, "Guide.md"), None);
        assert_eq!(
            case_collision(&mut seen, &lower, "guide.md").as_deref(),
            Some("Guide.md")
        );
    }
}
