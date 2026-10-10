use crate::resolve::relative_url;
use entwine_core::*;

/// A file ready for a caller-controlled output directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticFile {
    pub path: String,
    pub contents: Vec<u8>,
}

pub(crate) fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
pub(crate) fn frame(title: &str, route: &str, content: &str) -> String {
    frame_with(title, route, content, "")
}
/// A page frame with optional extra `<head>` markup (for the graph's deferred script).
pub(crate) fn frame_with(title: &str, route: &str, content: &str, head: &str) -> String {
    format!("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><meta name=\"generator\" content=\"Entwine {}\"><title>{}</title><link rel=\"stylesheet\" href=\"{}\">{head}</head><body><a class=\"skip\" href=\"#main\">Skip to content</a>{content}</body></html>", env!("CARGO_PKG_VERSION"), escape(title), escape(&relative_url(route, "/__entwine/style.css")))
}
pub(crate) fn nav(items: &[Navigation], current: &Route) -> String {
    let mut html = String::from("<ul>");
    let abbreviated = items.len() > 30;
    for (index, item) in items.iter().enumerate() {
        if abbreviated
            && index >= 20
            && !item
                .route
                .as_ref()
                .is_some_and(|route| current.as_str().starts_with(route.as_str()))
        {
            continue;
        }
        html.push_str("<li>");
        if let Some(route) = &item.route {
            html.push_str(&format!(
                "<a href=\"{}\"{}>{}</a>",
                escape(&relative_url(current.as_str(), route.as_str())),
                if route == current {
                    " aria-current=\"page\""
                } else {
                    ""
                },
                escape(&item.label)
            ));
        } else {
            html.push_str(&format!(
                "<span class=\"group\">{}</span>",
                escape(&item.label)
            ));
        }
        if !item.children.is_empty() {
            html.push_str(&nav(&item.children, current));
        }
        html.push_str("</li>");
    }
    if abbreviated {
        html.push_str(&format!(
            "<li><a href=\"{}\">All documents in Knowledge</a></li>",
            escape(&relative_url(current.as_str(), "/__entwine/knowledge/"))
        ));
    }
    html.push_str("</ul>");
    html
}

/// Relative page link prepared once for either page-shell renderer.
pub(crate) struct PreparedLink {
    pub href: String,
    pub label: String,
}

/// Heading data shared by the inline and side table of contents.
pub(crate) struct PreparedHeading {
    #[cfg(feature = "flowview-renderer")]
    pub id: String,
    pub href: String,
    pub text: String,
    pub class: String,
}

/// Metadata presentation shared by the built-in and Flowview page shells.
pub(crate) struct PreparedMetadata {
    pub class: String,
    pub label: String,
    pub is_role: bool,
}

pub(crate) fn page_metadata(page: &SitePage) -> Vec<PreparedMetadata> {
    let mut values = Vec::new();
    if page.role != KnowledgeRole::Other {
        values.push(PreparedMetadata {
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
        values.push(PreparedMetadata {
            class: String::new(),
            label: kind.to_owned(),
            is_role: false,
        });
    }
    if let Some(status) = page.metadata.status.as_deref() {
        values.push(PreparedMetadata {
            class: String::new(),
            label: status.to_owned(),
            is_role: false,
        });
    }
    values
}

pub(crate) fn page_headings(page: &SitePage) -> Vec<PreparedHeading> {
    page.headings
        .iter()
        .map(|heading| PreparedHeading {
            #[cfg(feature = "flowview-renderer")]
            id: heading.id.clone(),
            href: format!("#{}", heading.id),
            text: heading.text.clone(),
            class: format!("level-{}", heading.level),
        })
        .collect()
}

pub(crate) fn page_links(route: &str, links: &[PageReference]) -> Vec<PreparedLink> {
    links
        .iter()
        .map(|link| PreparedLink {
            href: relative_url(route, link.route.as_str()),
            label: link.title.clone(),
        })
        .collect()
}

pub(crate) fn page_views(
    site: &SiteModel,
    route: &str,
    graph_id: Option<&DocumentId>,
) -> Vec<PreparedLink> {
    let focus = graph_id
        .map(|id| format!("?focus={}", crate::resolve::encode_component(&id.0)))
        .unwrap_or_default();
    let mut views = vec![
        PreparedLink {
            href: relative_url(route, &site.knowledge_route),
            label: "Knowledge".into(),
        },
        PreparedLink {
            href: format!("{}{focus}", relative_url(route, &site.graph_route)),
            label: "Graph".into(),
        },
    ];
    if site.agents.is_some() {
        views.push(PreparedLink {
            href: relative_url(route, "/__entwine/agents/"),
            label: "Agents".into(),
        });
    }
    views
}

pub(crate) fn page_shows_title(page: &SitePage) -> bool {
    !page
        .headings
        .first()
        .is_some_and(|heading| heading.level == 1 && heading.text == page.title)
}

/// Render the site exclusively from its projection, without parsing Markdown.
pub fn render_site(site: &SiteModel) -> Vec<StaticFile> {
    let mut files = vec![StaticFile {
        path: "__entwine/style.css".into(),
        contents: include_bytes!("style.css").to_vec(),
    }];
    for page in &site.pages {
        let headings = page_headings(page);
        let toc = headings
            .iter()
            .map(|heading| {
                format!(
                    "<li class=\"{}\"><a href=\"{}\">{}</a></li>",
                    escape(&heading.class),
                    escape(&heading.href),
                    escape(&heading.text)
                )
            })
            .collect::<String>();
        let backlinks = related_section(
            "Referenced by",
            &page_links(page.route.as_str(), &page.backlinks),
        );
        let references = related_section(
            "References",
            &page_links(page.route.as_str(), &page.references),
        );
        let source = page
            .source_path
            .as_ref()
            .map(|path| match &page.source_url {
                Some(url) => format!(
                    "<p class=\"source\"><a href=\"{}\">Source: <code>{}</code></a></p>",
                    escape(url),
                    escape(path)
                ),
                None => format!(
                    "<p class=\"source\">Source: <code>{}</code></p>",
                    escape(path)
                ),
            })
            .unwrap_or_default();
        let metadata = page_metadata(page)
            .iter()
            .map(|item| {
                if item.is_role {
                    format!(
                        "<span class=\"{}\" title=\"Knowledge role\">{}</span>",
                        escape(&item.class),
                        escape(&item.label)
                    )
                } else {
                    format!("<span>{}</span>", escape(&item.label))
                }
            })
            .collect::<String>();
        let title = if page_shows_title(page) {
            format!("<h1>{}</h1>", escape(&page.title))
        } else {
            String::new()
        };
        let project = escape(
            site.navigation
                .first()
                .map_or("Documentation", |item| item.label.as_str()),
        );
        let navigation = nav(&site.navigation, &page.route);
        let views = page_views(site, page.route.as_str(), page.graph_id.as_ref());
        let views = format!(
            "<div class=\"views\" role=\"group\" aria-label=\"Entwine views\">{}</div>",
            views
                .iter()
                .map(|view| format!(
                    "<a href=\"{}\">{}</a>",
                    escape(&view.href),
                    escape(&view.label)
                ))
                .collect::<String>()
        );
        let home_href = escape(&relative_url(page.route.as_str(), "/"));
        let inline_toc = if page.headings.is_empty() {
            String::new()
        } else {
            format!("<details class=\"inline-toc\"><summary>On this page</summary><nav aria-label=\"Page headings\"><ul>{toc}</ul></nav></details>")
        };
        let content = format!("<details class=\"mobile-navigation\"><summary><span class=\"project-title\">{project}</span><span class=\"menu-label\">Menu</span></summary><nav aria-label=\"Mobile documentation\">{views}{navigation}</nav></details><div class=\"layout\"><aside class=\"sidebar\" aria-label=\"Project sidebar\"><a class=\"brand\" href=\"{home_href}\">{project}</a>{views}<nav aria-label=\"Documentation\">{navigation}</nav></aside><main id=\"main\" tabindex=\"-1\"><header><p class=\"metadata\">{metadata}</p>{title}</header>{inline_toc}<article>{}</article>{references}{backlinks}{source}<footer>Generated by Entwine · Markdown remains the source of truth.</footer></main><aside class=\"toc\" aria-label=\"Page outline\"><nav aria-label=\"On this page\"><h2>On this page</h2><ul>{toc}</ul></nav></aside></div>", page.html);
        files.push(StaticFile {
            path: format!("{}index.html", page.route.as_str().trim_start_matches('/')),
            contents: frame(&page.title, page.route.as_str(), &content).into_bytes(),
        });
    }
    files
}

fn related_section(title: &str, pages: &[PreparedLink]) -> String {
    if pages.is_empty() {
        return String::new();
    }
    let list = format!(
        "<ul>{}</ul>",
        pages
            .iter()
            .map(|p| format!(
                "<li><a href=\"{}\">{}</a></li>",
                escape(&p.href),
                escape(&p.label)
            ))
            .collect::<String>()
    );
    let body = if pages.len() > 12 {
        format!(
            "<details><summary>{} documents</summary>{list}</details>",
            pages.len()
        )
    } else {
        list
    };
    format!("<section class=\"backlinks\" aria-label=\"{title}\"><h2>{title}</h2>{body}</section>")
}

/// The generated Project Knowledge overview: which recommended areas are represented.
pub fn render_knowledge(site: &SiteModel) -> StaticFile {
    let route = site.knowledge_route.as_str();
    let link = |page: &PageReference| {
        format!(
            "<a href=\"{}\">{}</a>",
            escape(&relative_url(route, page.route.as_str())),
            escape(&page.title)
        )
    };
    let section = |title: &str, roles: &[KnowledgeRole], plural: bool, missing: &str| {
        let groups: Vec<_> = site
            .knowledge
            .iter()
            .filter(|g| roles.contains(&g.role))
            .collect();
        let body = if plural {
            let pages: Vec<_> = groups.iter().flat_map(|g| &g.pages).collect();
            if pages.is_empty() {
                format!("<p class=\"missing\"><span aria-hidden=\"true\">○</span> {missing}</p>")
            } else {
                format!(
                    "<p class=\"count\">{} document{}</p><ul>{}</ul>",
                    pages.len(),
                    if pages.len() == 1 { "" } else { "s" },
                    pages
                        .iter()
                        .map(|p| format!("<li>{}</li>", link(p)))
                        .collect::<String>()
                )
            }
        } else {
            let mut rows = String::new();
            for group in groups {
                if group.pages.is_empty() {
                    rows.push_str(&format!("<li class=\"missing\"><span aria-hidden=\"true\">○</span> {} <span class=\"state-text\">not found</span></li>", group.role.label()));
                } else {
                    rows.push_str(&format!(
                        "<li><span aria-hidden=\"true\">✓</span> {}: {}</li>",
                        group.role.label(),
                        group.pages.iter().map(&link).collect::<Vec<_>>().join(", ")
                    ));
                }
            }
            format!("<ul class=\"checklist\">{rows}</ul>")
        };
        format!("<section class=\"knowledge-section\"><h2>{title}</h2>{body}</section>")
    };
    let mut sections = vec![
        section(
            "Core",
            &[
                KnowledgeRole::Project,
                KnowledgeRole::Architecture,
                KnowledgeRole::State,
                KnowledgeRole::Roadmap,
            ],
            false,
            "",
        ),
        section(
            "Decisions",
            &[KnowledgeRole::Decision],
            true,
            "No decision documents found.",
        ),
        section(
            "Specifications",
            &[KnowledgeRole::Spec],
            true,
            "No specification documents found.",
        ),
    ];
    if site
        .knowledge
        .iter()
        .any(|g| g.role == KnowledgeRole::Other && !g.pages.is_empty())
    {
        sections.push(section(
            "Other knowledge",
            &[KnowledgeRole::Other],
            true,
            "",
        ));
    }
    if let Some(agents) = &site.agents {
        let count = |kind: ArtifactKind| agents.entries.iter().filter(|e| e.kind == kind).count();
        let (instructions, skills) = (
            count(ArtifactKind::AgentInstructions),
            count(ArtifactKind::Skill),
        );
        sections.push(format!(
            "<section class=\"knowledge-section\"><h2>Agent knowledge</h2><p class=\"count\">{instructions} instruction file{} · {skills} skill{}</p><p><a href=\"{}\">Browse agent instructions, skills, and scopes</a></p></section>",
            if instructions == 1 { "" } else { "s" },
            if skills == 1 { "" } else { "s" },
            escape(&relative_url(route, "/__entwine/agents/"))
        ));
    }
    let repository = if site.repository_reference_count == 0 {
        String::new()
    } else {
        format!("<p class=\"note\">{} references to repository files, kept separate from document relationships.</p>", site.repository_reference_count)
    };
    let agents_nav = if site.agents.is_some() {
        "<a href=\"../agents/\">Agents</a>"
    } else {
        ""
    };
    let content = format!("<main id=\"main\" tabindex=\"-1\" class=\"graph-page knowledge-page\"><header class=\"graph-header\"><nav class=\"graph-nav\" aria-label=\"Entwine views\"><a href=\"../../\">Documentation</a><a href=\"../graph/\">Graph</a>{agents_nav}</nav><h1>Project knowledge</h1><p>What project knowledge exists. The <a href=\"../graph/\">graph</a> shows how it is connected.</p></header>{}<p class=\"note\">Recommended areas follow the Entwine Knowledge Convention. They are suggestions, not requirements; this page shows presence, not quality. Presence is not quality.</p>{repository}</main>", sections.join(""));
    StaticFile {
        path: "__entwine/knowledge/index.html".into(),
        contents: frame("Project knowledge", route, &content).into_bytes(),
    }
}
