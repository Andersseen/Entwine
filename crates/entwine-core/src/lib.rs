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
    pub backlinks: Vec<PageReference>,
}
/// Complete input for a replaceable site renderer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteModel {
    pub pages: Vec<SitePage>,
    pub navigation: Vec<Navigation>,
    pub graph_route: String,
}
/// One graph node per document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: DocumentId,
    pub label: String,
    pub route: Route,
    pub metadata: DocumentMetadata,
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
