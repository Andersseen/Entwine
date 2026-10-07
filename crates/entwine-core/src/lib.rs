//! Pure, serializable project knowledge. No I/O or rendering lives here.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// A stable identifier: the normalized path relative to `docs/`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DocumentId(pub String);

/// A normalized, root-relative, trailing-slash documentation route.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Route(String);

impl Route {
    /// The documentation entry route, also used for generated directory indexes.
    pub fn home() -> Self {
        Self("/".into())
    }
    /// Derive a route from a normalized Markdown path, rejecting unsafe names.
    pub fn from_source(source: &str) -> Result<Self, String> {
        let source = source.replace('\\', "/");
        let parts: Vec<_> = source.split('/').collect();
        if parts.iter().any(|p| {
            p.is_empty()
                || *p == "."
                || *p == ".."
                || p.chars().any(|c| c.is_control() || ":?#%".contains(c))
        }) {
            return Err(format!("Invalid documentation path: {source}"));
        }
        let stem = source
            .strip_suffix(".md")
            .ok_or_else(|| "Documentation paths must end in .md".to_string())?;
        if stem
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        {
            return Err(format!("Invalid documentation filename: {source}"));
        }
        let stem = if stem == "index" {
            ""
        } else {
            stem.strip_suffix("/index").unwrap_or(stem)
        };
        if stem.split('/').next() == Some("__entwine") {
            return Err("The __entwine route is reserved".into());
        }
        Ok(Self(if stem.is_empty() {
            "/".into()
        } else {
            format!("/{stem}/")
        }))
    }
    /// A generated Entwine view route below `__entwine/`, such as `__entwine/agents/skills/x`.
    /// Segments must already be sanitized; anything unsafe is rejected.
    pub fn generated(path: &str) -> Result<Self, String> {
        let trimmed = path.trim_matches('/');
        let mut parts = trimmed.split('/');
        if parts.next() != Some("__entwine")
            || trimmed.split('/').any(|p| {
                p.is_empty()
                    || p == "."
                    || p == ".."
                    || p.starts_with('.')
                    || !p
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
            })
        {
            return Err(format!("Invalid generated route: {path}"));
        }
        Ok(Self(format!("/{trimmed}/")))
    }
    /// The route as a UTF-8 string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Optional author-supplied classification; values are open-ended strings.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub status: Option<String>,
}

/// Derived Entwine classification. Author metadata (`type`) stays open-ended;
/// this closed set only names the roles the Knowledge Convention recognizes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeRole {
    Project,
    Architecture,
    State,
    Roadmap,
    Decision,
    Spec,
    Other,
}
impl KnowledgeRole {
    /// Stable machine name, also the recognized frontmatter `type` value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Architecture => "architecture",
            Self::State => "state",
            Self::Roadmap => "roadmap",
            Self::Decision => "decision",
            Self::Spec => "spec",
            Self::Other => "other",
        }
    }
    /// Human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Project => "Project",
            Self::Architecture => "Architecture",
            Self::State => "Current state",
            Self::Roadmap => "Roadmap",
            Self::Decision => "Decision",
            Self::Spec => "Specification",
            Self::Other => "Other",
        }
    }
    /// Deterministic ordering for projections: convention order, `Other` last.
    pub fn rank(self) -> u8 {
        self as u8
    }
    /// Recognize a frontmatter `type` value (case-insensitive, with plurals).
    pub fn from_type(value: &str) -> Option<Self> {
        Some(match value.trim().to_ascii_lowercase().as_str() {
            "project" => Self::Project,
            "architecture" => Self::Architecture,
            "state" => Self::State,
            "roadmap" => Self::Roadmap,
            "decision" | "decisions" | "adr" => Self::Decision,
            "spec" | "specs" | "specification" => Self::Spec,
            _ => return None,
        })
    }
    /// Role implied by a canonical path relative to `docs/`. Names match case-insensitively,
    /// because real repositories commonly use `STATE.md` or `ARCHITECTURE.md`.
    pub fn from_path(id: &str) -> Option<Self> {
        let id = id.to_ascii_lowercase();
        Some(match id.as_str() {
            "index.md" => Self::Project,
            "architecture.md" => Self::Architecture,
            "state.md" => Self::State,
            "roadmap.md" => Self::Roadmap,
            // A section landing page explains the directory; it is not itself a decision or spec.
            _ if id.ends_with("/index.md") && id.matches('/').count() == 1 => return None,
            _ if id.starts_with("decisions/") && id.ends_with(".md") => Self::Decision,
            _ if id.starts_with("specs/") && id.ends_with(".md") => Self::Spec,
            _ => return None,
        })
    }
}

/// Classify a document: recognized explicit `type`, then canonical path, then `Other`.
/// The second value is a diagnostic message when the two sources clearly conflict.
pub fn classify(id: &str, kind: Option<&str>) -> (KnowledgeRole, Option<String>) {
    let explicit = kind.and_then(KnowledgeRole::from_type);
    let canonical = KnowledgeRole::from_path(id);
    let conflict = match (explicit, canonical) {
        (Some(e), Some(c)) if e != c => Some(format!(
            "type `{}` conflicts with the canonical path, which implies `{}`; using `{}`",
            kind.unwrap_or_default(),
            c.as_str(),
            e.as_str()
        )),
        _ => None,
    };
    (
        explicit.or(canonical).unwrap_or(KnowledgeRole::Other),
        conflict,
    )
}

/// Optional `entwine.toml`. Every field defaults to the zero-config behavior:
/// compile `docs/`, discover nothing else, publish nothing agent-facing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub discovery: DiscoveryConfig,
    pub site: SiteConfig,
}
/// Which repository knowledge artifacts are discovered (modeled in context and checks).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DiscoveryConfig {
    /// `AGENTS.md`, `CLAUDE.md`, `GEMINI.md` at any depth.
    pub agent_instructions: bool,
    /// `SKILL.md` manifests at any depth.
    pub skills: bool,
}
/// Publication policy: discovery never implies publication.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SiteConfig {
    /// Include discovered agent-facing artifacts in the generated site and its graph.
    pub include_agent_knowledge: bool,
}
impl Config {
    pub fn discovers_agent_knowledge(&self) -> bool {
        self.discovery.agent_instructions || self.discovery.skills
    }
}

/// What kind of repository artifact a document is. Orthogonal to [`KnowledgeRole`], which
/// says what a document *means*; this says what *kind of file* it is.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    /// Durable project documentation under `docs/`.
    #[default]
    Documentation,
    /// Agent-facing instructions such as `AGENTS.md`, `CLAUDE.md`, or `GEMINI.md`.
    AgentInstructions,
    /// A skill manifest (`SKILL.md`) and its enclosing skill folder.
    Skill,
}
impl ArtifactKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Documentation => "documentation",
            Self::AgentInstructions => "agent_instructions",
            Self::Skill => "skill",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Documentation => "Documentation",
            Self::AgentInstructions => "Agent instructions",
            Self::Skill => "Skill",
        }
    }
    /// Agent-facing artifacts are the ones governed by publication policy.
    pub fn is_agent_facing(self) -> bool {
        self != Self::Documentation
    }
}

/// The file-name convention an agent-facing artifact follows. These name conventions,
/// not products: Entwine models them and never resolves or runs anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentConvention {
    AgentsMd,
    ClaudeMd,
    GeminiMd,
    SkillMd,
}
impl AgentConvention {
    /// Recognize a convention from an exact file name (case-sensitive, as on case-sensitive hosts).
    pub fn from_file_name(name: &str) -> Option<Self> {
        Some(match name {
            "AGENTS.md" => Self::AgentsMd,
            "CLAUDE.md" => Self::ClaudeMd,
            "GEMINI.md" => Self::GeminiMd,
            "SKILL.md" => Self::SkillMd,
            _ => return None,
        })
    }
    pub fn file_name(self) -> &'static str {
        match self {
            Self::AgentsMd => "AGENTS.md",
            Self::ClaudeMd => "CLAUDE.md",
            Self::GeminiMd => "GEMINI.md",
            Self::SkillMd => "SKILL.md",
        }
    }
    pub fn kind(self) -> ArtifactKind {
        if self == Self::SkillMd {
            ArtifactKind::Skill
        } else {
            ArtifactKind::AgentInstructions
        }
    }
}

/// What proves that one skill manifest is only an exposure of another. Evidence is declared or
/// structural, never inferred from similar text. More bases may be added when real
/// repositories show them; consumers should treat unknown values as "an exposure".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExposureBasis {
    /// The exposure links to the canonical `SKILL.md`, declares the same `name`, and carries no
    /// colocated files of its own.
    DeclaredLink,
}
impl ExposureBasis {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DeclaredLink => "declared_link",
        }
    }
}

/// Structure of a discovered agent-facing artifact. Absent on ordinary documentation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentDetails {
    pub convention: AgentConvention,
    /// Repository-relative path of the file, always `/`-separated.
    pub path: String,
    /// Instruction scope: the project-relative directory the file sits in
    /// (`""` is the project root). Structural only; no provider precedence is simulated.
    pub scope: Option<String>,
    /// `name` frontmatter of a skill, if present.
    pub name: Option<String>,
    /// `description` frontmatter of a skill, if present.
    pub description: Option<String>,
    /// Repository-relative directory of a skill (like `path`).
    pub skill_directory: Option<String>,
    /// Files colocated with a skill manifest, relative to the skill directory. Never read or run.
    pub resources: Vec<String>,
    /// Canonical skill this manifest is a provable exposure (bridge, alias) of. Origin is
    /// orthogonal to the artifact kind: an exposure is still a skill.
    #[serde(default)]
    pub exposure_of: Option<DocumentId>,
    /// Why `exposure_of` holds.
    #[serde(default)]
    pub exposure_basis: Option<ExposureBasis>,
    /// Manifests that are exposures of this one, sorted.
    #[serde(default)]
    pub exposures: Vec<DocumentId>,
}

/// Document id prefix that keeps agent artifacts distinct from `docs/` paths.
/// `:` cannot occur in a documentation path, so ids never collide.
pub const REPOSITORY_ID_PREFIX: &str = "repo:";

/// An extracted heading with the exact generated HTML identifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Heading {
    pub level: u8,
    pub text: String,
    pub id: String,
}

/// One source link. Unresolved/external links have no document target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    pub destination: String,
    pub line: usize,
    pub target: Option<DocumentId>,
    pub href: String,
}

/// A safe file reference inside the repository, distinct from knowledge relations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryReference {
    pub path: String,
    pub source: DocumentId,
    pub line: usize,
    pub destination: String,
}

/// Provider-neutral source URL prefix supplied by the I/O layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositorySource {
    pub file_base_url: String,
    /// Optional inventory of files present at the source ref; uncommitted files degrade to paths.
    pub files: Option<BTreeSet<String>>,
}

/// Canonical document: source content, compiled body, and extracted structure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document {
    pub id: DocumentId,
    pub route: Route,
    pub title: String,
    pub metadata: DocumentMetadata,
    pub role: KnowledgeRole,
    pub headings: Vec<Heading>,
    pub links: Vec<Link>,
    pub anchors: Vec<String>,
    pub repository_references: Vec<RepositoryReference>,
    #[serde(default)]
    pub artifact: ArtifactKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<AgentDetails>,
    pub content: String,
    pub html: String,
}
impl Document {
    /// Repository path of the source file given the documentation root prefix.
    pub fn source_path(&self, docs_prefix: &str) -> String {
        match &self.agent {
            Some(agent) => agent.path.clone(),
            None => format!("{docs_prefix}/{}", self.id.0),
        }
    }
}

/// The MVP has one explicit relationship kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationKind {
    References,
}

/// Unique document-to-document relationship, independent of link frequency.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Relation {
    pub source: DocumentId,
    pub target: DocumentId,
    pub kind: RelationKind,
}

/// Canonical compilation result, ordered by source path.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeBase {
    pub documents: Vec<Document>,
    pub relations: Vec<Relation>,
}
impl KnowledgeBase {
    /// Derive unique incoming documents from the canonical relations.
    pub fn backlinks(&self, target: &DocumentId) -> Vec<DocumentId> {
        self.relations
            .iter()
            .filter(|r| &r.target == target)
            .map(|r| r.source.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
}

/// Presentation-neutral validation severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}
/// An actionable source diagnostic. Lines are one-based.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub source: String,
    pub line: Option<usize>,
    pub message: String,
}

/// One recommended knowledge area and the documents that represent it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoverageArea {
    pub role: KnowledgeRole,
    pub documents: Vec<DocumentId>,
}
impl CoverageArea {
    pub fn is_present(&self) -> bool {
        !self.documents.is_empty()
    }
}
/// Recommended roles, in convention order. `Other` is not a recommended area.
pub const RECOMMENDED_ROLES: [KnowledgeRole; 6] = [
    KnowledgeRole::Project,
    KnowledgeRole::Architecture,
    KnowledgeRole::State,
    KnowledgeRole::Roadmap,
    KnowledgeRole::Decision,
    KnowledgeRole::Spec,
];
/// Which recommended areas are represented. This is presence, never quality.
pub fn coverage(documents: &[Document]) -> Vec<CoverageArea> {
    RECOMMENDED_ROLES
        .iter()
        .map(|role| CoverageArea {
            role: *role,
            documents: documents
                .iter()
                .filter(|d| d.role == *role)
                .map(|d| d.id.clone())
                .collect(),
        })
        .collect()
}

/// Navigation groups follow the directory tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Navigation {
    pub label: String,
    pub route: Option<Route>,
    pub children: Vec<Navigation>,
}
/// A small title/route reference, used by backlinks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageReference {
    pub title: String,
    pub route: Route,
}
/// A renderer-ready page; rendering does not inspect source Markdown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SitePage {
    pub route: Route,
    pub title: String,
    pub html: String,
    pub headings: Vec<Heading>,
    pub metadata: DocumentMetadata,
    pub role: KnowledgeRole,
    pub artifact: ArtifactKind,
    pub graph_id: Option<DocumentId>,
    pub backlinks: Vec<PageReference>,
    pub references: Vec<PageReference>,
    pub source_path: Option<String>,
    pub source_url: Option<String>,
}
/// Complete input for a replaceable site renderer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteModel {
    pub pages: Vec<SitePage>,
    pub navigation: Vec<Navigation>,
    pub graph_route: String,
    pub knowledge_route: String,
    pub knowledge: Vec<KnowledgeGroup>,
    pub repository_reference_count: usize,
    /// Agent-facing knowledge published on the site; `None` when publication is off.
    pub agents: Option<AgentsModel>,
}
/// A link to another artifact or document from an Agents page, with what it points at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentLink {
    pub title: String,
    pub route: Route,
    pub kind: ArtifactKind,
    /// Repository path of the source file.
    pub path: String,
}
/// One agent-facing artifact, ready for the generated Agents views.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentEntry {
    pub id: DocumentId,
    pub title: String,
    pub route: Route,
    pub kind: ArtifactKind,
    pub details: AgentDetails,
    /// Documents and artifacts this one references / is referenced by.
    pub references: Vec<AgentLink>,
    pub backlinks: Vec<AgentLink>,
    /// The canonical skill this entry is an exposure of, if proven.
    pub canonical: Option<AgentLink>,
    /// Manifests that only expose this skill.
    pub exposures: Vec<AgentLink>,
    pub html: String,
    pub source_url: Option<String>,
    /// Resource path (relative to the skill directory) and an optional source link.
    pub resource_links: Vec<(String, Option<String>)>,
}
/// The generated Agents section, deterministic and grouped.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentsModel {
    pub entries: Vec<AgentEntry>,
}
/// Pages of one knowledge role, for the generated Project Knowledge overview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeGroup {
    pub role: KnowledgeRole,
    pub pages: Vec<PageReference>,
}
/// One graph node per document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: DocumentId,
    pub label: String,
    pub route: Route,
    pub metadata: DocumentMetadata,
    pub role: KnowledgeRole,
    pub artifact: ArtifactKind,
    /// Repository path of the source file.
    pub path: String,
    /// Instruction scope (`""` is the whole project), for agent instructions.
    pub scope: Option<String>,
    /// Directory containing the skill folder (for example `.agents/skills`), for skills.
    pub location: Option<String>,
    /// Canonical skill this node merely exposes. Such nodes are collapsed by default.
    pub exposure_of: Option<DocumentId>,
    /// Number of exposures of this skill.
    pub exposures: usize,
}
/// A repository file referenced by documents; a secondary, optional graph node.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GraphFile {
    pub path: String,
    pub referenced_by: Vec<DocumentId>,
}
/// Deterministic graph projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphModel {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<Relation>,
    pub files: Vec<GraphFile>,
}
/// Structured context is versioned separately from the package version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextModel {
    pub schema_version: String,
    pub documents: Vec<ContextDocument>,
    pub relationships: Vec<Relation>,
}
/// Source content plus resolved structure for downstream programs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextDocument {
    pub id: DocumentId,
    pub title: String,
    pub route: Route,
    pub metadata: DocumentMetadata,
    pub role: KnowledgeRole,
    pub headings: Vec<Heading>,
    pub links: Vec<Link>,
    pub backlinks: Vec<DocumentId>,
    pub anchors: Vec<String>,
    pub repository_references: Vec<RepositoryReference>,
    pub artifact: ArtifactKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<AgentDetails>,
    pub content: String,
}
/// Resolve the title consistently, including for index pages.
pub fn document_title(metadata: &DocumentMetadata, headings: &[Heading], source: &str) -> String {
    metadata
        .title
        .clone()
        .or_else(|| {
            headings
                .iter()
                .find(|h| h.level == 1)
                .map(|h| h.text.clone())
        })
        .unwrap_or_else(|| {
            let name = source
                .rsplit('/')
                .next()
                .unwrap_or(source)
                .trim_end_matches(".md");
            humanize(name)
        })
}
/// A readable filesystem label without custom ordering rules.
pub fn humanize(value: &str) -> String {
    value
        .replace(['-', '_'], " ")
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            chars
                .next()
                .map(|c| c.to_uppercase().collect::<String>() + chars.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn routes_are_normalized_and_safe() {
        for (source, route) in [
            ("index.md", "/"),
            ("specs/auth.md", "/specs/auth/"),
            ("specs\\index.md", "/specs/"),
        ] {
            assert_eq!(Route::from_source(source).unwrap().as_str(), route);
        }
        for source in [
            "../bad.md",
            "/bad.md",
            "a//b.md",
            "__entwine/graph.md",
            "a?.md",
            ".md",
            "..md",
            "...md",
            "a/...md",
        ] {
            assert!(Route::from_source(source).is_err());
        }
    }
    #[test]
    fn titles_use_metadata_then_h1_then_filename() {
        let headings = vec![Heading {
            level: 1,
            text: "Heading".into(),
            id: "heading".into(),
        }];
        let metadata = DocumentMetadata {
            title: Some("Explicit".into()),
            ..Default::default()
        };
        assert_eq!(document_title(&metadata, &headings, "a.md"), "Explicit");
        assert_eq!(
            document_title(&DocumentMetadata::default(), &headings, "a.md"),
            "Heading"
        );
        assert_eq!(
            document_title(&DocumentMetadata::default(), &[], "static-output.md"),
            "Static Output"
        );
    }

    #[test]
    fn roles_prefer_recognized_type_then_path_then_other() {
        use KnowledgeRole::*;
        assert_eq!(classify("architecture.md", None).0, Architecture);
        assert_eq!(classify("decisions/a/b.md", None).0, Decision);
        assert_eq!(classify("specs/x.md", Some("guide")).0, Spec);
        assert_eq!(
            classify("design/system.md", Some("Architecture")).0,
            Architecture
        );
        assert_eq!(classify("foo.md", Some("guide")).0, Other);
        assert_eq!(classify("foo.md", None), (Other, None));
        // Conflicts warn but the explicit type wins.
        let (role, conflict) = classify("architecture.md", Some("roadmap"));
        assert_eq!(role, Roadmap);
        assert!(conflict.unwrap().contains("canonical path"));
        // Agreement and unknown types are not conflicts.
        assert!(classify("state.md", Some("state")).1.is_none());
        assert!(classify("state.md", Some("guide")).1.is_none());
        // Only the docs-root index is the project entry point.
        assert_eq!(classify("specs/index.md", None).0, Other);
        assert_eq!(classify("specs/auth/index.md", None).0, Spec);
        assert_eq!(classify("decisions/index.md", Some("decision")).0, Decision);
        assert_eq!(classify("sub/index.md", None).0, Other);
        // Real repositories use uppercase names; no renames required.
        assert_eq!(classify("STATE.md", None).0, State);
        assert_eq!(classify("ARCHITECTURE.md", None).0, Architecture);
        assert_eq!(classify("Specs/001-x.md", None).0, Spec);
    }

    #[test]
    fn backlinks_are_unique_and_sorted() {
        let target = DocumentId("target.md".into());
        let knowledge = KnowledgeBase {
            documents: Vec::new(),
            relations: ["z.md", "a.md", "z.md"]
                .into_iter()
                .map(|source| Relation {
                    source: DocumentId(source.into()),
                    target: target.clone(),
                    kind: RelationKind::References,
                })
                .collect(),
        };
        assert_eq!(
            knowledge.backlinks(&target),
            vec![DocumentId("a.md".into()), DocumentId("z.md".into())]
        );
    }
}
