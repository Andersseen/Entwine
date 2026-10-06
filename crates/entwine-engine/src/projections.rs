use crate::{render::escape, resolve::relative_url};
use entwine_core::*;
use std::collections::BTreeMap;

/// Context schema `0.3` adds explicit anchors and repository references.
pub const CONTEXT_SCHEMA_VERSION: &str = "0.3";

fn knowledge_groups(knowledge: &KnowledgeBase) -> Vec<KnowledgeGroup> {
    let mut roles = RECOMMENDED_ROLES.to_vec();
    if knowledge
        .documents
        .iter()
        .any(|d| d.role == KnowledgeRole::Other)
    {
        roles.push(KnowledgeRole::Other);
    }
    roles
        .into_iter()
        .map(|role| KnowledgeGroup {
            role,
            pages: knowledge
                .documents
                .iter()
                .filter(|d| d.role == role)
                .map(|d| PageReference {
                    title: d.title.clone(),
                    route: d.route.clone(),
                })
                .collect(),
        })
        .collect()
}

#[derive(Default)]
struct Tree {
    page: Option<PageReference>,
    children: BTreeMap<String, Tree>,
}
fn navigation(tree: Tree, prefix: &str) -> Vec<Navigation> {
    tree.children
        .into_iter()
        .map(|(name, branch)| {
            let path = format!("{prefix}{name}/");
            Navigation {
                label: branch
                    .page
                    .as_ref()
                    .map_or_else(|| humanize(&name), |p| p.title.clone()),
                route: branch
                    .page
                    .as_ref()
                    .map(|p| p.route.clone())
                    .or_else(|| Route::from_source(&format!("{path}index.md")).ok()),
                children: navigation(branch, &path),
            }
        })
        .collect()
}

pub(crate) fn project(knowledge: &KnowledgeBase) -> (SiteModel, GraphModel, ContextModel) {
    let mut tree = Tree::default();
    let mut home = None;
    for document in &knowledge.documents {
        let reference = PageReference {
            title: document.title.clone(),
            route: document.route.clone(),
        };
        if document.route.as_str() == "/" {
            home = Some(Navigation {
                label: reference.title,
                route: Some(reference.route),
                children: Vec::new(),
            });
            continue;
        }
        let mut branch = &mut tree;
        for part in document.route.as_str().trim_matches('/').split('/') {
            branch = branch.children.entry(part.into()).or_default();
        }
        branch.page = Some(reference);
    }
    merge_directory_entries(&mut tree);
    let mut nav = navigation(tree, "");
    let needs_index = home.is_none();
    nav.insert(
        0,
        home.unwrap_or_else(|| Navigation {
            label: "Documentation".into(),
            route: Some(Route::home()),
            children: Vec::new(),
        }),
    );
    let mut pages: Vec<SitePage> = knowledge
        .documents
        .iter()
        .map(|d| SitePage {
            route: d.route.clone(),
            title: d.title.clone(),
            html: d.html.clone(),
            headings: d.headings.clone(),
            metadata: d.metadata.clone(),
            role: d.role,
            references: knowledge
                .relations
                .iter()
                .filter(|r| r.source == d.id)
                .filter_map(|r| knowledge.documents.iter().find(|d| d.id == r.target))
                .map(|d| PageReference {
                    title: d.title.clone(),
                    route: d.route.clone(),
                })
                .collect(),
            source_path: Some(format!("docs/{}", d.id.0)),
            source_url: None,
            backlinks: knowledge
                .backlinks(&d.id)
                .iter()
                .filter_map(|id| knowledge.documents.iter().find(|d| &d.id == id))
                .map(|d| PageReference {
                    title: d.title.clone(),
                    route: d.route.clone(),
                })
                .collect(),
        })
        .collect();
    for directory in directory_routes(knowledge) {
        if knowledge
            .documents
            .iter()
            .any(|p| p.route.as_str().eq_ignore_ascii_case(directory.as_str()))
        {
            continue;
        }
        let html = format!(
            "<ul>{}</ul>",
            knowledge
                .documents
                .iter()
                .filter(|d| d.route.as_str().starts_with(directory.as_str()))
                .map(|d| format!(
                    "<li><a href=\"{}\">{}</a></li>",
                    escape(&relative_url(directory.as_str(), d.route.as_str())),
                    escape(&d.title)
                ))
                .collect::<String>()
        );
        pages.push(SitePage {
            route: directory.clone(),
            title: humanize(
                directory
                    .as_str()
                    .trim_matches('/')
                    .rsplit('/')
                    .next()
                    .unwrap_or("Documentation"),
            ),
            html,
            headings: Vec::new(),
            metadata: DocumentMetadata::default(),
            role: KnowledgeRole::Other,
            backlinks: Vec::new(),
            references: Vec::new(),
            source_path: None,
            source_url: None,
        });
    }
    if needs_index {
        let html = format!(
            "<p>Project documentation compiled from Markdown.</p><ul>{}</ul>",
            knowledge
                .documents
                .iter()
                .map(|document| format!(
                    "<li><a href=\"{}\">{}</a></li>",
                    escape(&relative_url("/", document.route.as_str())),
                    escape(&document.title)
                ))
                .collect::<String>()
        );
        pages.insert(
            0,
            SitePage {
                route: Route::home(),
                title: "Documentation".into(),
                html,
                headings: Vec::new(),
                metadata: DocumentMetadata::default(),
                role: KnowledgeRole::Project,
                backlinks: Vec::new(),
                references: Vec::new(),
                source_path: None,
                source_url: None,
            },
        );
    }
    let graph = GraphModel {
        nodes: knowledge
            .documents
            .iter()
            .map(|d| GraphNode {
                id: d.id.clone(),
                label: d.title.clone(),
                route: d.route.clone(),
                metadata: d.metadata.clone(),
                role: d.role,
            })
            .collect(),
        edges: knowledge.relations.clone(),
    };
    let context = ContextModel {
        schema_version: CONTEXT_SCHEMA_VERSION.into(),
        documents: knowledge
            .documents
            .iter()
            .map(|d| ContextDocument {
                id: d.id.clone(),
                title: d.title.clone(),
                route: d.route.clone(),
                metadata: d.metadata.clone(),
                role: d.role,
                headings: d.headings.clone(),
                links: d.links.clone(),
                backlinks: knowledge.backlinks(&d.id),
                anchors: d.anchors.clone(),
                repository_references: d.repository_references.clone(),
                content: d.content.clone(),
            })
            .collect(),
        relationships: knowledge.relations.clone(),
    };
    (
        SiteModel {
            pages,
            navigation: nav,
            graph_route: "/__entwine/graph/".into(),
            knowledge_route: "/__entwine/knowledge/".into(),
            knowledge: knowledge_groups(knowledge),
            repository_reference_count: knowledge
                .documents
                .iter()
                .map(|d| d.repository_references.len())
                .sum(),
        },
        graph,
        context,
    )
}

/// Serialize the versioned context model deterministically.
pub fn context_json(context: &ContextModel) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(context)
}

/// Human-readable context including complete Markdown content and relations.
pub fn context_markdown(context: &ContextModel) -> String {
    let project = context
        .documents
        .iter()
        .find(|d| d.route.as_str() == "/")
        .map_or("Project", |d| d.title.as_str());
    let mut output = format!(
        "# Project: {project}\n\nContext schema: {}\n\n## Documents\n\n",
        context.schema_version
    );
    for document in &context.documents {
        output.push_str(&format!(
            "- {} — {} ({}) [{}]\n",
            document.title,
            document.route.as_str(),
            document.id.0,
            document.role.as_str()
        ));
    }
    output.push_str("\n## Relationships\n\n");
    for relation in &context.relationships {
        output.push_str(&format!(
            "- {} references {}\n",
            relation.source.0, relation.target.0
        ));
    }
    output.push_str("\n## Repository references\n\n");
    for document in &context.documents {
        for reference in &document.repository_references {
            output.push_str(&format!(
                "- {}:{} → {} ({})\n",
                reference.source.0, reference.line, reference.path, reference.destination
            ));
        }
    }
    output.push_str("\n## Content\n");
    for document in &context.documents {
        output.push_str(&format!(
            "\n### {}\n\nSource: {}\nRoute: {}\nRole: {}\nType: {}\nStatus: {}\nBacklinks: {}\n\n",
            document.title,
            document.id.0,
            document.route.as_str(),
            document.role.as_str(),
            document.metadata.kind.as_deref().unwrap_or("unspecified"),
            document.metadata.status.as_deref().unwrap_or("unspecified"),
            document
                .backlinks
                .iter()
                .map(|id| id.0.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
        output.push_str(&document.content);
        output.push('\n');
    }
    output
}

fn directory_routes(knowledge: &KnowledgeBase) -> std::collections::BTreeSet<Route> {
    knowledge
        .documents
        .iter()
        .flat_map(|d| {
            let parts: Vec<_> = d.id.0.split('/').collect();
            (1..parts.len()).filter_map(move |i| {
                Route::from_source(&format!("{}/index.md", parts[..i].join("/"))).ok()
            })
        })
        .collect()
}
pub(crate) fn source_links(
    site: &mut SiteModel,
    knowledge: &KnowledgeBase,
    docs: &std::path::Path,
    repository: &std::path::Path,
    source: Option<&RepositorySource>,
) {
    let prefix = docs
        .strip_prefix(repository)
        .unwrap_or(docs)
        .to_string_lossy()
        .replace('\\', "/");
    for page in &mut site.pages {
        if let Some(document) = knowledge.documents.iter().find(|d| d.route == page.route) {
            let path = format!("{prefix}/{}", document.id.0);
            page.source_path = Some(path.clone());
            page.source_url = source
                .filter(|s| s.files.as_ref().is_none_or(|files| files.contains(&path)))
                .map(|s| crate::resolve::source_url(s, &path, ""));
        }
    }
}

/// An authored route such as /ROADMAP/ also owns a case-equivalent directory
/// landing on portable hosts. Merge its leaf navigation into the directory group.
fn merge_directory_entries(tree: &mut Tree) {
    let groups: Vec<_> = tree
        .children
        .iter()
        .filter(|(_, branch)| !branch.children.is_empty())
        .map(|(name, _)| name.clone())
        .collect();
    for name in groups {
        let leaf = tree
            .children
            .iter()
            .find(|(other, branch)| {
                *other != &name
                    && other.eq_ignore_ascii_case(&name)
                    && branch.children.is_empty()
                    && branch.page.is_some()
            })
            .map(|(other, _)| other.clone());
        if let Some(leaf) = leaf {
            let page = tree.children.remove(&leaf).and_then(|branch| branch.page);
            if let Some(group) = tree.children.get_mut(&name) {
                if group.page.is_none() {
                    group.page = page;
                }
            }
        }
    }
    for branch in tree.children.values_mut() {
        merge_directory_entries(branch);
    }
}

#[cfg(test)]
mod portable_directories {
    use super::*;
    #[test]
    fn generated_case_variants_remain_visible_to_collision_validation() {
        let documents = ["A/one.md", "a/two.md"]
            .into_iter()
            .map(|source| {
                crate::markdown::finish(crate::markdown::parse(
                    source,
                    Route::from_source(source).unwrap(),
                    "# Document",
                    &mut Vec::new(),
                ))
            })
            .collect();
        let knowledge = KnowledgeBase {
            documents,
            relations: Vec::new(),
        };
        let site = project(&knowledge).0;
        assert!(site.pages.iter().any(|p| p.route.as_str() == "/A/"));
        assert!(site.pages.iter().any(|p| p.route.as_str() == "/a/"));
    }
}
