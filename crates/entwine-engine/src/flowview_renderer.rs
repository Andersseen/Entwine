//! Experimental Flowview page-shell renderer.
//!
//! Entwine prepares every route and label before passing this narrow JSON view
//! to Flowview. The Markdown body is trusted because Entwine rendered it from
//! Markdown using its existing raw-HTML policy; all other template values are
//! escaped by ordinary interpolation.

use crate::{
    render::{nav, StaticFile},
    resolve::relative_url,
};
use entwine_core::{KnowledgeRole, SiteModel};
use flowview_compiler::{compile_static, CompiledStaticTemplate, StaticCompileOptions};
use serde_json::{json, Value};
use std::io;

const PAGE_TEMPLATE: &str = include_str!("templates/page.flow");

fn compile_template() -> io::Result<CompiledStaticTemplate> {
    compile_static(
        PAGE_TEMPLATE,
        StaticCompileOptions::default().with_filename("entwine-page.flow"),
    )
    .map_err(|diagnostics| {
        let message = diagnostics
            .iter()
            .map(|diagnostic| {
                format!(
                    "{}:{}:{}: {}",
                    diagnostic.code.as_deref().unwrap_or("Flowview"),
                    diagnostic.line,
                    diagnostic.column,
                    diagnostic.message
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        io::Error::other(format!(
            "Internal Flowview page template failed to compile:\n{message}"
        ))
    })
}

pub(crate) fn render_site(site: &SiteModel) -> io::Result<Vec<StaticFile>> {
    // Compile exactly once for this complete site render, then reuse the
    // immutable template for all pages.
    let template = compile_template()?;
    let mut files = vec![StaticFile {
        path: "__entwine/style.css".into(),
        contents: include_bytes!("style.css").to_vec(),
    }];

    for page in &site.pages {
        let route = page.route.as_str();
        let project = site
            .navigation
            .first()
            .map_or("Documentation", |item| item.label.as_str());
        let metadata = metadata_values(
            page.role,
            page.metadata.kind.as_deref(),
            page.metadata.status.as_deref(),
        );
        let headings = page
            .headings
            .iter()
            .map(|heading| {
                json!({
                    "id": heading.id,
                    "href": format!("#{}", heading.id),
                    "text": heading.text,
                    "class": format!("level-{}", heading.level),
                })
            })
            .collect::<Vec<_>>();
        let backlinks = page
            .backlinks
            .iter()
            .map(|link| {
                json!({
                    "href": relative_url(route, link.route.as_str()),
                    "label": link.title,
                })
            })
            .collect::<Vec<_>>();
        let references = page
            .references
            .iter()
            .map(|link| {
                json!({
                    "href": relative_url(route, link.route.as_str()),
                    "label": link.title,
                })
            })
            .collect::<Vec<_>>();
        let focus = page
            .graph_id
            .as_ref()
            .map(|id| format!("?focus={}", crate::resolve::encode_component(&id.0)))
            .unwrap_or_default();
        let mut views = vec![
            json!({"href": relative_url(route, &site.knowledge_route), "label": "Knowledge"}),
            json!({"href": format!("{}{focus}", relative_url(route, &site.graph_route)), "label": "Graph"}),
        ];
        if site.agents.is_some() {
            views.push(
                json!({"href": relative_url(route, "/__entwine/agents/"), "label": "Agents"}),
            );
        }
        let source = page.source_path.as_ref().map(|path| {
            json!({
                "path": path,
                "url": page.source_url,
                "has_url": page.source_url.is_some(),
            })
        });
        let model = json!({
            "site": {
                "generator": format!("Entwine {}", env!("CARGO_PKG_VERSION")),
                "project": project,
            },
            "assets": {
                "stylesheet": relative_url(route, "/__entwine/style.css"),
            },
            "page": {
                "title": page.title,
                "body_html": page.html,
                "show_title": !page.headings.first().is_some_and(|heading| heading.level == 1 && heading.text == page.title),
                "has_headings": !headings.is_empty(),
                "metadata": metadata,
                "headings": headings,
                "references": references,
                "has_references": !page.references.is_empty(),
                "references_collapsible": page.references.len() > 12,
                "backlinks": backlinks,
                "has_backlinks": !page.backlinks.is_empty(),
                "backlinks_collapsible": page.backlinks.len() > 12,
                "source": source,
                "has_source": page.source_path.is_some(),
            },
            "views": views,
            "home_href": relative_url(route, "/"),
            // Navigation labels and hrefs are escaped by Entwine's existing
            // helper before this trusted, host-generated fragment is inserted.
            "navigation_html": nav(&site.navigation, &page.route),
        });
        let html = template.render(&model).map_err(|diagnostics| {
            let message = diagnostics
                .iter()
                .map(|diagnostic| {
                    format!(
                        "{}:{}:{}: {}",
                        diagnostic.code.as_deref().unwrap_or("Flowview"),
                        diagnostic.line,
                        diagnostic.column,
                        diagnostic.message
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            io::Error::other(format!(
                "Internal Flowview renderer failed for {}:\n{message}",
                page.route.as_str()
            ))
        })?;
        files.push(StaticFile {
            path: format!("{}index.html", route.trim_start_matches('/')),
            contents: html.into_bytes(),
        });
    }
    Ok(files)
}

fn metadata_values(role: KnowledgeRole, kind: Option<&str>, status: Option<&str>) -> Vec<Value> {
    let mut values = Vec::new();
    if role != KnowledgeRole::Other {
        values.push(json!({
            "class": format!("role role-{}", role.as_str()),
            "label": role.label(),
            "is_role": true,
        }));
    }
    if let Some(kind) = kind.filter(|kind| !kind.eq_ignore_ascii_case(role.as_str())) {
        values.push(json!({"class": "", "label": kind, "is_role": false}));
    }
    if let Some(status) = status {
        values.push(json!({"class": "", "label": status, "is_role": false}));
    }
    values
}
