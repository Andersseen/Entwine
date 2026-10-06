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
    pub content: String,
    pub html: String,
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
    pub backlinks: Vec<PageReference>,
}
/// Complete input for a replaceable site renderer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteModel {
    pub pages: Vec<SitePage>,
    pub navigation: Vec<Navigation>,
    pub graph_route: String,
    pub knowledge_route: String,
    pub knowledge: Vec<KnowledgeGroup>,
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
}
/// Deterministic graph projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphModel {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<Relation>,
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
