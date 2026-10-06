use crate::graph_layout::{edge_path, layout};
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
fn frame(title: &str, route: &str, content: &str) -> String {
    format!("<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><meta name=\"generator\" content=\"Entwine {}\"><title>{}</title><link rel=\"stylesheet\" href=\"{}\"></head><body><a class=\"skip\" href=\"#main\">Skip to content</a>{content}</body></html>", env!("CARGO_PKG_VERSION"), escape(title), escape(&relative_url(route, "/__entwine/style.css")))
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
        let graph_href = escape(&relative_url(page.route.as_str(), &site.graph_route));
        let knowledge_href = escape(&relative_url(page.route.as_str(), &site.knowledge_route));
        let views = format!("<div class=\"views\" role=\"group\" aria-label=\"Entwine views\"><a href=\"{knowledge_href}\">Knowledge</a><a href=\"{graph_href}\">Graph</a></div>");
        let home_href = escape(&relative_url(page.route.as_str(), "/"));
        let inline_toc = if page.headings.is_empty() {
            String::new()
        } else {
            format!("<details class=\"inline-toc\"><summary>On this page</summary><nav aria-label=\"Page headings\"><ul>{toc}</ul></nav></details>")
        };
        let content = format!("<details class=\"mobile-navigation\"><summary><span class=\"project-title\">{project}</span><span class=\"menu-label\">Menu</span></summary><nav aria-label=\"Mobile documentation\">{views}{navigation}</nav></details><div class=\"layout\"><aside class=\"sidebar\"><a class=\"brand\" href=\"{home_href}\">{project}</a>{views}<nav aria-label=\"Documentation\">{navigation}</nav></aside><main id=\"main\" tabindex=\"-1\"><header><p class=\"metadata\">{metadata}</p>{title}</header>{inline_toc}<article>{}</article>{references}{backlinks}{source}<footer>Generated by Entwine · Markdown remains the source of truth.</footer></main><aside class=\"toc\"><nav aria-label=\"On this page\"><h2>On this page</h2><ul>{toc}</ul></nav></aside></div>", page.html);
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

/// Deterministic SVG with linked nodes, directional edges, and a readable edge list.
pub fn render_graph(graph: &GraphModel) -> StaticFile {
    let route = "/__entwine/graph/";
    let layout = layout(graph);
    let size = layout.size;
    let label_size = if graph.nodes.len() <= 9 { 26 } else { 22 };
    let mut svg = format!("<svg class=\"graph\" xmlns=\"http://www.w3.org/2000/svg\" width=\"{size}\" height=\"{size}\" viewBox=\"0 0 {size} {size}\" role=\"group\" aria-labelledby=\"graph-title graph-description\"><title id=\"graph-title\">Project knowledge graph</title><desc id=\"graph-description\">{} linked documents and {} directed references. Small graphs center the most connected document; larger graphs group documents by knowledge role. A text list of all relationships follows.</desc><defs><marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"5\" markerHeight=\"5\" orient=\"auto\"><path d=\"M 0 0 L 10 5 L 0 10 z\"/></marker></defs>", graph.nodes.len(), graph.edges.len());
    for edge in &graph.edges {
        if let (Some(source), Some(target)) = (
            layout.positions.get(&edge.source),
            layout.positions.get(&edge.target),
        ) {
            let source_radius = if layout.hub.as_ref() == Some(&edge.source) {
                25.0
            } else {
                19.0
            };
            let target_radius = if layout.hub.as_ref() == Some(&edge.target) {
                25.0
            } else {
                19.0
            };
            let path = edge_path(*source, *target, source_radius, target_radius);
            svg.push_str(&format!("<path class=\"edge\" data-source=\"{}\" data-target=\"{}\" d=\"{path}\" marker-end=\"url(#arrow)\"><title>{} references {}</title></path>", escape(&edge.source.0), escape(&edge.target.0), escape(&edge.source.0), escape(&edge.target.0)));
        }
    }
    for node in &graph.nodes {
        if let Some(point) = layout.positions.get(&node.id) {
            let radius = if layout.hub.as_ref() == Some(&node.id) {
                25
            } else {
                19
            };
            let label = graph_label(&node.label)
                .iter()
                .enumerate()
                .map(|(index, line)| {
                    format!(
                        "<tspan x=\"{}\" dy=\"{}\">{}</tspan>",
                        point.x,
                        if index == 0 { 0 } else { 25 },
                        escape(line)
                    )
                })
                .collect::<String>();
            svg.push_str(&format!("<a class=\"graph-node\" href=\"{}\" data-node=\"{}\" data-role=\"{}\" aria-label=\"{}\"><title>{}</title><circle class=\"node\" cx=\"{}\" cy=\"{}\" r=\"{radius}\"/><circle class=\"node-dot\" cx=\"{}\" cy=\"{}\" r=\"4\"/><text class=\"node-label\" text-anchor=\"middle\" x=\"{}\" y=\"{}\">{label}</text></a>", escape(&relative_url(route, node.route.as_str())), escape(&node.id.0), node.role.as_str(), escape(&node_name(node)), escape(&node_name(node)), point.x, point.y, point.x, point.y, point.x, point.y + f64::from(radius) + 29.0));
        }
    }
    svg.push_str("</svg>");
    let references = graph
        .edges
        .iter()
        .filter_map(|edge| {
            let source = graph.nodes.iter().find(|n| n.id == edge.source)?;
            let target = graph.nodes.iter().find(|n| n.id == edge.target)?;
            Some(format!(
                "<li><a href=\"{}\">{}</a> references <a href=\"{}\">{}</a></li>",
                escape(&relative_url(route, source.route.as_str())),
                escape(&source.label),
                escape(&relative_url(route, target.route.as_str())),
                escape(&target.label)
            ))
        })
        .collect::<String>();
    let mut ordered: Vec<_> = graph.nodes.iter().collect();
    ordered.sort_by(|a, b| (a.role.rank(), &a.id).cmp(&(b.role.rank(), &b.id)));
    let nodes = ordered
        .iter()
        .map(|n| {
            format!(
                "<li><a href=\"{}\">{}</a>{}</li>",
                escape(&relative_url(route, n.route.as_str())),
                escape(&n.label),
                if n.role == KnowledgeRole::Other {
                    String::new()
                } else {
                    format!(" <span class=\"role-tag\">{}</span>", n.role.label())
                }
            )
        })
        .collect::<String>();
    let large = graph.nodes.len() > 50 || graph.edges.len() > 200;
    let grouped = RECOMMENDED_ROLES
        .iter()
        .chain(std::iter::once(&KnowledgeRole::Other))
        .map(|role| {
            let documents = ordered
                .iter()
                .filter(|n| n.role == *role)
                .map(|n| {
                    format!(
                        "<li><a href=\"{}\">{}</a> <code>{}</code></li>",
                        escape(&relative_url(route, n.route.as_str())),
                        escape(&n.label),
                        escape(&n.id.0)
                    )
                })
                .collect::<String>();
            if documents.is_empty() {
                String::new()
            } else {
                format!(
                    "<section><h2>{}</h2><ul>{documents}</ul></section>",
                    role.label()
                )
            }
        })
        .collect::<String>();
    let overview = if large {
        format!("<p class=\"note\">Large graph: use the complete role-grouped document and relationship index below. Your browser’s Find searches every title and path.</p>{grouped}")
    } else {
        String::new()
    };
    let visual_open = if large { "" } else { " open" };
    let content = format!("<main id=\"main\" tabindex=\"-1\" class=\"graph-page\"><header class=\"graph-header\"><nav class=\"graph-nav\" aria-label=\"Entwine views\"><a href=\"../../\">Documentation</a><a href=\"../knowledge/\">Knowledge</a></nav><h1>Project graph</h1><p>{} documents · {} relationships</p></header>{overview}<details{visual_open}><summary>Visual graph</summary><section class=\"graph-view\" aria-label=\"Graph view\"><p class=\"graph-hint\">Select a document to open it. Arrows point to referenced pages.</p><input class=\"graph-toggle\" type=\"checkbox\" id=\"large-graph\"><label class=\"graph-control\" for=\"large-graph\">Large view</label><div class=\"graph-scroll\" tabindex=\"0\" aria-label=\"Document graph; scroll in large view\" style=\"--graph-size: {size}px; --label-size: {label_size}px\">{svg}</div></section></details><div class=\"graph-index\"><details><summary>Documents ({})</summary><ul>{nodes}</ul></details><details open><summary>Relationships ({})</summary><ul>{references}</ul></details></div></main>", graph.nodes.len(), graph.edges.len(), graph.nodes.len(), graph.edges.len());
    StaticFile {
        path: "__entwine/graph/index.html".into(),
        contents: frame("Project graph", route, &content).into_bytes(),
    }
}

fn node_name(node: &GraphNode) -> String {
    if node.role == KnowledgeRole::Other {
        node.label.clone()
    } else {
        format!("{} ({})", node.label, node.role.label())
    }
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
    let repository = if site.repository_reference_count == 0 {
        String::new()
    } else {
        format!("<p class=\"note\">{} references to repository files, kept separate from document relationships.</p>", site.repository_reference_count)
    };
    let content = format!("<main id=\"main\" tabindex=\"-1\" class=\"graph-page knowledge-page\"><header class=\"graph-header\"><nav class=\"graph-nav\" aria-label=\"Entwine views\"><a href=\"../../\">Documentation</a><a href=\"../graph/\">Graph</a></nav><h1>Project knowledge</h1><p>What project knowledge exists. The <a href=\"../graph/\">graph</a> shows how it is connected.</p></header>{}<p class=\"note\">Recommended areas follow the Entwine Knowledge Convention. They are suggestions, not requirements; this page shows presence, not quality. Presence is not quality.</p>{repository}</main>", sections.join(""));
    StaticFile {
        path: "__entwine/knowledge/index.html".into(),
        contents: frame("Project knowledge", route, &content).into_bytes(),
    }
}

fn graph_label(label: &str) -> Vec<String> {
    let chars: Vec<_> = label.trim().chars().collect();
    if chars.len() <= 16 {
        return vec![label.trim().into()];
    }
    let split = chars[..16]
        .iter()
        .rposition(|c| c.is_whitespace())
        .filter(|index| *index >= 5)
        .unwrap_or(16);
    let first: String = chars[..split].iter().collect();
    let rest: String = chars[split..].iter().collect::<String>().trim().into();
    let second = if rest.chars().count() > 16 {
        rest.chars().take(15).collect::<String>() + "…"
    } else {
        rest
    };
    vec![first, second]
}
