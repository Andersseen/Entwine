//! The ordinary documentation page renderer.
//!
//! Entwine resolves every route, label, heading, and relationship into the `SiteModel`
//! and prepares them here as a small typed view model. Flowview only turns the canonical
//! `templates/page.flow` shell into static HTML, natively, with no runtime in the output.
//!
//! Escaping contract: every value below is interpolated with `{{ }}` and therefore
//! escaped by Flowview. Exactly two fields are inserted raw with `{{{ }}}`, both produced
//! by Entwine: `page.body_html` (Markdown rendered under Entwine's raw-HTML policy) and
//! `navigation_html` (built by `render::nav`, which escapes every label and href).

use crate::{
    render::{nav, StaticFile},
    resolve::{encode_component, relative_url},
};
use entwine_core::{DocumentId, KnowledgeRole, PageReference, SiteModel, SitePage};
use flowview_compiler::{compile_static, CompiledStaticTemplate, Diagnostic, StaticCompileOptions};
use serde::Serialize;
use std::io;

const PAGE_TEMPLATE: &str = include_str!("templates/page.flow");
const TEMPLATE_NAME: &str = "entwine-page.flow";
const COLLAPSE_ABOVE: usize = 12;

#[derive(Serialize)]
struct PageTemplateContext<'a> {
    site: SiteContext<'a>,
    assets: AssetContext,
    page: PageContext<'a>,
    views: Vec<Link>,
    home_href: String,
    /// Trusted: escaped by Entwine's navigation renderer.
    navigation_html: String,
}

#[derive(Serialize)]
struct SiteContext<'a> {
    generator: String,
    project: &'a str,
}

#[derive(Serialize)]
struct AssetContext {
    stylesheet: String,
}

#[derive(Serialize)]
struct PageContext<'a> {
    title: &'a str,
    /// Trusted: Markdown rendered by Entwine under its raw-HTML policy.
    body_html: &'a str,
    show_title: bool,
    metadata: Vec<MetadataItem>,
    has_headings: bool,
    headings: Vec<Heading<'a>>,
    has_references: bool,
    references_collapsible: bool,
    references: Vec<Link>,
    has_backlinks: bool,
    backlinks_collapsible: bool,
    backlinks: Vec<Link>,
    has_source: bool,
    source: Option<Source<'a>>,
}

#[derive(Serialize)]
struct MetadataItem {
    class: String,
    label: String,
    is_role: bool,
}

#[derive(Serialize)]
struct Heading<'a> {
    id: &'a str,
    href: String,
    text: &'a str,
    class: String,
}

#[derive(Serialize)]
struct Link {
    href: String,
    label: String,
}

#[derive(Serialize)]
struct Source<'a> {
    path: &'a str,
    url: Option<&'a str>,
    has_url: bool,
}

/// Render every ordinary documentation page, plus the shared stylesheet.
///
/// The page template is compiled once and reused for every route.
pub fn render_site(site: &SiteModel) -> io::Result<Vec<StaticFile>> {
    let template = compile_static(
        PAGE_TEMPLATE,
        StaticCompileOptions::default().with_filename(TEMPLATE_NAME),
    )
    .map_err(|diagnostics| {
        io::Error::other(format!(
            "Internal page template failed to compile:\n{}",
            describe(&diagnostics)
        ))
    })?;
    render_pages(site, &template)
}

fn render_pages(
    site: &SiteModel,
    template: &CompiledStaticTemplate,
) -> io::Result<Vec<StaticFile>> {
    let mut files = Vec::with_capacity(site.pages.len() + 1);
    files.push(StaticFile {
        path: "__entwine/style.css".into(),
        contents: include_bytes!("style.css").to_vec(),
    });
    let project = site
        .navigation
        .first()
        .map_or("Documentation", |item| item.label.as_str());
    let generator = format!("Entwine {}", env!("CARGO_PKG_VERSION"));

    for page in &site.pages {
        let route = page.route.as_str();
        let context = page_context(site, page, project, &generator);
        let value = serde_json::to_value(&context)
            .map_err(|error| io::Error::other(format!("Cannot prepare {route}: {error}")))?;
        let html = template.render(&value).map_err(|diagnostics| {
            io::Error::other(format!(
                "Internal page renderer failed for {route}:\n{}",
                describe(&diagnostics)
            ))
        })?;
        files.push(StaticFile {
            path: format!("{}index.html", route.trim_start_matches('/')),
            contents: html.into_bytes(),
        });
    }
    Ok(files)
}

fn page_context<'a>(
    site: &'a SiteModel,
    page: &'a SitePage,
    project: &'a str,
    generator: &str,
) -> PageTemplateContext<'a> {
    let route = page.route.as_str();
    let headings: Vec<_> = page
        .headings
        .iter()
        .map(|heading| Heading {
            id: &heading.id,
            href: format!("#{}", heading.id),
            text: &heading.text,
            class: format!("level-{}", heading.level),
        })
        .collect();
    PageTemplateContext {
        site: SiteContext {
            generator: generator.to_owned(),
            project,
        },
        assets: AssetContext {
            stylesheet: relative_url(route, "/__entwine/style.css"),
        },
        page: PageContext {
            title: &page.title,
            body_html: &page.html,
            show_title: shows_title(page),
            metadata: metadata(page),
            has_headings: !headings.is_empty(),
            headings,
            has_references: !page.references.is_empty(),
            references_collapsible: page.references.len() > COLLAPSE_ABOVE,
            references: links(route, &page.references),
            has_backlinks: !page.backlinks.is_empty(),
            backlinks_collapsible: page.backlinks.len() > COLLAPSE_ABOVE,
            backlinks: links(route, &page.backlinks),
            has_source: page.source_path.is_some(),
            source: page.source_path.as_deref().map(|path| Source {
                path,
                url: page.source_url.as_deref(),
                has_url: page.source_url.is_some(),
            }),
        },
        views: views(site, route, page.graph_id.as_ref()),
        home_href: relative_url(route, "/"),
        navigation_html: nav(&site.navigation, &page.route),
    }
}

fn metadata(page: &SitePage) -> Vec<MetadataItem> {
    let mut values = Vec::new();
    if page.role != KnowledgeRole::Other {
        values.push(MetadataItem {
            class: format!("role role-{}", page.role.as_str()),
            label: page.role.label().to_owned(),
            is_role: true,
        });
    }
    if let Some(kind) = page
        .metadata
        .kind
        .as_deref()
        .filter(|kind| !kind.eq_ignore_ascii_case(page.role.as_str()))
    {
        values.push(MetadataItem {
            class: String::new(),
            label: kind.to_owned(),
            is_role: false,
        });
    }
    if let Some(status) = page.metadata.status.as_deref() {
        values.push(MetadataItem {
            class: String::new(),
            label: status.to_owned(),
            is_role: false,
        });
    }
    values
}

fn links(route: &str, references: &[PageReference]) -> Vec<Link> {
    references
        .iter()
        .map(|reference| Link {
            href: relative_url(route, reference.route.as_str()),
            label: reference.title.clone(),
        })
        .collect()
}

fn views(site: &SiteModel, route: &str, graph_id: Option<&DocumentId>) -> Vec<Link> {
    let focus = graph_id
        .map(|id| format!("?focus={}", encode_component(&id.0)))
        .unwrap_or_default();
    let mut views = vec![
        Link {
            href: relative_url(route, &site.knowledge_route),
            label: "Knowledge".into(),
        },
        Link {
            href: format!("{}{focus}", relative_url(route, &site.graph_route)),
            label: "Graph".into(),
        },
    ];
    if site.agents.is_some() {
        views.push(Link {
            href: relative_url(route, "/__entwine/agents/"),
            label: "Agents".into(),
        });
    }
    views
}

/// The visible `<h1>` is omitted when the document's own first heading already says it.
fn shows_title(page: &SitePage) -> bool {
    !page
        .headings
        .first()
        .is_some_and(|heading| heading.level == 1 && heading.text == page.title)
}

fn describe(diagnostics: &[Diagnostic]) -> String {
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
