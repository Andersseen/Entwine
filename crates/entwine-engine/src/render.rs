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
fn nav(items: &[Navigation], current: &Route) -> String {
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

/// Render the site exclusively from its projection, without parsing Markdown.
pub fn render_site(site: &SiteModel) -> Vec<StaticFile> {
    let mut files = vec![StaticFile {
        path: "__entwine/style.css".into(),
        contents: include_bytes!("style.css").to_vec(),
    }];
    for page in &site.pages {
        let toc = page
            .headings
            .iter()
            .map(|h| {
                format!(
                    "<li class=\"level-{}\"><a href=\"#{}\">{}</a></li>",
                    h.level,
                    escape(&h.id),
                    escape(&h.text)
                )
            })
            .collect::<String>();
        let backlinks = related_section("Referenced by", &page.backlinks, &page.route);
        let references = related_section("References", &page.references, &page.route);
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
        let role = if page.role == KnowledgeRole::Other {
            String::new()
        } else {
            format!(
                "<span class=\"role role-{}\" title=\"Knowledge role\">{}</span>",
                page.role.as_str(),
                page.role.label()
            )
        };
        let raw_type = page
            .metadata
            .kind
            .as_deref()
            .filter(|kind| !kind.eq_ignore_ascii_case(page.role.as_str()));
        let metadata = role
            + &[raw_type, page.metadata.status.as_deref()]
                .into_iter()
                .flatten()
                .map(|value| format!("<span>{}</span>", escape(value)))
                .collect::<String>();
        let title = if page
            .headings
            .first()
            .is_some_and(|heading| heading.level == 1 && heading.text == page.title)
        {
            String::new()
        } else {
            format!("<h1>{}</h1>", escape(&page.title))
        };
        let project = escape(
            site.navigation
                .first()
                .map_or("Documentation", |item| item.label.as_str()),
        );
        let navigation = nav(&site.navigation, &page.route);
        let focus = page
            .graph_id
            .as_ref()
            .map(|id| format!("?focus={}", crate::resolve::encode_component(&id.0)))
            .unwrap_or_default();
        let graph_href = escape(&format!(
            "{}{focus}",
            relative_url(page.route.as_str(), &site.graph_route)
        ));
        let knowledge_href = escape(&relative_url(page.route.as_str(), &site.knowledge_route));
        let agents_link = if site.agents.is_some() {
            format!(
                "<a href=\"{}\">Agents</a>",
                escape(&relative_url(page.route.as_str(), "/__entwine/agents/"))
            )
        } else {
            String::new()
        };
        let views = format!("<div class=\"views\" role=\"group\" aria-label=\"Entwine views\"><a href=\"{knowledge_href}\">Knowledge</a><a href=\"{graph_href}\">Graph</a>{agents_link}</div>");
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

fn related_section(title: &str, pages: &[PageReference], route: &Route) -> String {
    if pages.is_empty() {
        return String::new();
    }
    let list = format!(
        "<ul>{}</ul>",
        pages
            .iter()
            .map(|p| format!(
                "<li><a href=\"{}\">{}</a></li>",
                escape(&relative_url(route.as_str(), p.route.as_str())),
                escape(&p.title)
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
