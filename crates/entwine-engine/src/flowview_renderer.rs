//! Experimental Flowview page-shell renderer.
//!
//! Entwine prepares every route and label before passing the small typed
//! [`PageContext`] to Flowview. The Markdown body is trusted because Entwine
//! rendered it from Markdown using its existing raw-HTML policy, and the
//! recursive navigation is trusted because Entwine's `nav` helper escapes every
//! label and href. All other template values are escaped by ordinary
//! interpolation.

use crate::{
    render::{
        nav, page_headings, page_links, page_metadata, page_shows_title, page_views,
        PreparedHeading, PreparedLink, PreparedMetadata, StaticFile, COLLAPSED_LINKS_AFTER,
    },
    resolve::relative_url,
};
use entwine_core::SiteModel;
use flowview_compiler::{compile_static, CompiledStaticTemplate, Diagnostic, StaticCompileOptions};
use serde::Serialize;
use std::io;

const PAGE_TEMPLATE: &str = include_str!("templates/page.flow");

/// The template input for one ordinary documentation page. It is internal to
/// this adapter and is not a public schema.
#[derive(Serialize)]
struct PageContext<'a> {
    site: SiteContext<'a>,
    assets: AssetsContext,
    page: PageData<'a>,
    views: Vec<PreparedLink>,
    home_href: String,
    /// Trusted host HTML: Flowview has no recursion or partials.
    navigation_html: String,
}

#[derive(Serialize)]
struct SiteContext<'a> {
    generator: &'static str,
    project: &'a str,
}

#[derive(Serialize)]
struct AssetsContext {
    stylesheet: String,
}

#[derive(Serialize)]
struct PageData<'a> {
    title: &'a str,
    /// Trusted host HTML rendered from Markdown by Entwine.
    body_html: &'a str,
    show_title: bool,
    metadata: Vec<PreparedMetadata>,
    headings: Vec<PreparedHeading>,
    references: Vec<PreparedLink>,
    references_collapsible: bool,
    backlinks: Vec<PreparedLink>,
    backlinks_collapsible: bool,
    source: Option<SourceContext<'a>>,
}

#[derive(Serialize)]
struct SourceContext<'a> {
    path: &'a str,
    url: Option<&'a str>,
}

fn diagnostics_message(diagnostics: &[Diagnostic]) -> String {
    diagnostics
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
        .join("\n")
}

fn compile_template() -> io::Result<CompiledStaticTemplate> {
    compile_static(
        PAGE_TEMPLATE,
        StaticCompileOptions::default().with_filename("entwine-page.flow"),
    )
    .map_err(|diagnostics| {
        io::Error::other(format!(
            "Internal Flowview page template failed to compile:\n{}",
            diagnostics_message(&diagnostics)
        ))
    })
}

pub(crate) fn render_site(site: &SiteModel) -> io::Result<Vec<StaticFile>> {
    // Compile exactly once for this complete site render, then reuse the
    // immutable template for all pages.
    let template = compile_template()?;
    let generator = concat!("Entwine ", env!("CARGO_PKG_VERSION"));
    let project = site
        .navigation
        .first()
        .map_or("Documentation", |item| item.label.as_str());
    let mut files = vec![StaticFile {
        path: "__entwine/style.css".into(),
        contents: include_bytes!("style.css").to_vec(),
    }];

    for page in &site.pages {
        let route = page.route.as_str();
        let context = PageContext {
            site: SiteContext { generator, project },
            assets: AssetsContext {
                stylesheet: relative_url(route, "/__entwine/style.css"),
            },
            page: PageData {
                title: &page.title,
                body_html: &page.html,
                show_title: page_shows_title(page),
                metadata: page_metadata(page),
                headings: page_headings(page),
                references: page_links(route, &page.references),
                references_collapsible: page.references.len() > COLLAPSED_LINKS_AFTER,
                backlinks: page_links(route, &page.backlinks),
                backlinks_collapsible: page.backlinks.len() > COLLAPSED_LINKS_AFTER,
                source: page.source_path.as_deref().map(|path| SourceContext {
                    path,
                    url: page.source_url.as_deref(),
                }),
            },
            views: page_views(site, route, page.graph_id.as_ref()),
            home_href: relative_url(route, "/"),
            navigation_html: nav(&site.navigation, &page.route),
        };
        // Ergonomic only: this still materializes a serde_json::Value per page.
        let html = template
            .render_serializable(&context)
            .map_err(|diagnostics| {
                io::Error::other(format!(
                    "Internal Flowview renderer failed for {route}:\n{}",
                    diagnostics_message(&diagnostics)
                ))
            })?;
        files.push(StaticFile {
            path: format!("{}index.html", route.trim_start_matches('/')),
            contents: html.into_bytes(),
        });
    }
    Ok(files)
}
