use crate::{render::escape, resolve::relative_url};
use entwine_core::*;
use std::collections::BTreeMap;

/// Context schema `0.2` adds `role` to every document; all `0.1` fields are unchanged.
pub const CONTEXT_SCHEMA_VERSION: &str = "0.2";

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
fn navigation(tree: Tree) -> Vec<Navigation> {
    tree.children
        .into_iter()
        .map(|(name, branch)| Navigation {
            label: branch
                .page
                .as_ref()
                .map_or_else(|| humanize(&name), |p| p.title.clone()),
            route: branch.page.as_ref().map(|p| p.route.clone()),
            children: navigation(branch),
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
    let mut nav = navigation(tree);
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
