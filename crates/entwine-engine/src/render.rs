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
