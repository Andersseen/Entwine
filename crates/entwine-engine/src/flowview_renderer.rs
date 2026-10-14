//! Experimental Flowview page-shell renderer.
//!
//! Entwine prepares every route and label before passing this narrow JSON view
//! to Flowview. The Markdown body is trusted because Entwine rendered it from
//! Markdown using its existing raw-HTML policy; all other template values are
//! escaped by ordinary interpolation.

use crate::{
    render::{
        nav, page_headings, page_links, page_metadata, page_shows_title, page_views, StaticFile,
    },
    resolve::relative_url,
};
use entwine_core::SiteModel;
use flowview_compiler::{compile_static, CompiledStaticTemplate, StaticCompileOptions};
use serde_json::json;
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
        let metadata = page_metadata(page)
            .iter()
            .map(|item| {
                json!({
                    "class": item.class,
                    "label": item.label,
                    "is_role": item.is_role,
                })
            })
            .collect::<Vec<_>>();
        let headings = page_headings(page)
            .iter()
            .map(|heading| {
                json!({
                    "id": heading.id,
                    "href": heading.href,
                    "text": heading.text,
                    "class": heading.class,
                })
            })
            .collect::<Vec<_>>();
        let backlinks = page_links(route, &page.backlinks)
            .iter()
            .map(|link| {
                json!({
                    "href": link.href,
                    "label": link.label,
                })
            })
            .collect::<Vec<_>>();
        let references = page_links(route, &page.references)
            .iter()
            .map(|link| {
                json!({
                    "href": link.href,
                    "label": link.label,
                })
            })
            .collect::<Vec<_>>();
        let views = page_views(site, route, page.graph_id.as_ref())
            .iter()
            .map(|view| json!({"href": view.href, "label": view.label}))
            .collect::<Vec<_>>();
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
                "show_title": page_shows_title(page),
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
