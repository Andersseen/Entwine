//! Generated Agents views: what agent-facing knowledge exists, where it lives, what scope it
//! covers, and how it connects to documentation. Rendered only when publication is enabled.
use crate::render::{escape, frame, StaticFile};
use crate::resolve::{encode_component, relative_url};
use entwine_core::*;
use std::collections::BTreeSet;

const HOME: &str = "/__entwine/agents/";

fn scope_label(scope: &str) -> &str {
    if scope.is_empty() {
        "Project root"
    } else {
        scope
    }
}

fn shell(route: &str, title: &str, body: &str, active: &str) -> StaticFile {
    let to = |target: &str| escape(&relative_url(route, target));
    let tab = |name: &str, target: &str| {
        format!(
            "<a href=\"{}\"{}>{name}</a>",
            to(target),
            if name == active {
                " aria-current=\"page\""
            } else {
                ""
            }
        )
    };
    let content = format!(
        "<main id=\"main\" tabindex=\"-1\" class=\"graph-page agents-page\"><header class=\"graph-header\"><nav class=\"graph-nav\" aria-label=\"Entwine views\"><a href=\"{}\">Documentation</a><a href=\"{}\">Knowledge</a><a href=\"{}\">Graph</a><a href=\"{}\" aria-current=\"page\">Agents</a></nav><h1>{}</h1><nav class=\"graph-nav agents-tabs\" aria-label=\"Agent knowledge\">{}{}{}{}</nav></header>{body}<footer class=\"source\">Entwine discovers and models these files. It does not run, evaluate, or resolve them for any tool.</footer></main>",
        to("/"),
        to("/__entwine/knowledge/"),
        to("/__entwine/graph/"),
        to(HOME),
        escape(title),
        tab("Overview", HOME),
        tab("Instructions", "/__entwine/agents/instructions/"),
        tab("Skills", "/__entwine/agents/skills/"),
        tab("Scopes", "/__entwine/agents/scopes/"),
    );
    StaticFile {
        path: format!("{}index.html", route.trim_start_matches('/')),
        contents: frame(title, route, &content).into_bytes(),
    }
}

fn link(from: &str, route: &Route, title: &str) -> String {
    format!(
        "<a href=\"{}\">{}</a>",
        escape(&relative_url(from, route.as_str())),
        escape(title)
    )
}
fn links(from: &str, pages: &[PageReference]) -> String {
    pages
        .iter()
        .map(|p| format!("<li>{}</li>", link(from, &p.route, &p.title)))
        .collect()
}
fn plural(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

fn instruction_item(from: &str, entry: &AgentEntry) -> String {
    format!(
        "<li>{} <span class=\"role-tag\">{}</span> <code>{}</code></li>",
        link(from, &entry.route, &entry.title),
        entry.details.convention.file_name(),
        escape(&entry.details.path)
    )
}

fn skill_card(from: &str, entry: &AgentEntry) -> String {
    let details = &entry.details;
    format!(
        "<li class=\"agent-card\"><h3>{}</h3><p>{}</p><p><code>{}</code></p><p>{} · {}</p></li>",
        link(from, &entry.route, &entry.title),
        escape(
            details
                .description
                .as_deref()
                .unwrap_or("No description in frontmatter.")
        ),
        escape(details.skill_directory.as_deref().unwrap_or("")),
        plural(details.resources.len(), "resource", "resources"),
        plural(entry.references.len(), "reference", "references"),
    )
}

pub fn render_agents(site: &SiteModel) -> Vec<StaticFile> {
    let Some(agents) = &site.agents else {
        return Vec::new();
    };
    let instructions: Vec<&AgentEntry> = agents
        .entries
        .iter()
        .filter(|e| e.kind == ArtifactKind::AgentInstructions)
        .collect();
    let skills: Vec<&AgentEntry> = agents
        .entries
        .iter()
        .filter(|e| e.kind == ArtifactKind::Skill)
        .collect();
    let scopes: BTreeSet<&str> = instructions
        .iter()
        .filter_map(|e| e.details.scope.as_deref())
        .collect();
    let mut files = Vec::new();

    // Overview
    let mut body = format!("<p class=\"note\">Agent-facing knowledge found in this repository: {} in {}, and {}. Documentation lives in the <a href=\"{}\">main site</a>; this section keeps instructions and skills structured instead of mixing them in.</p>",
        plural(instructions.len(), "instruction file", "instruction files"),
        plural(scopes.len(), "scope", "scopes"),
        plural(skills.len(), "skill", "skills"),
        escape(&relative_url(HOME, "/")));
    if !instructions.is_empty() {
        body.push_str(&format!(
            "<section><h2>Instructions</h2><ul>{}</ul></section>",
            instructions
                .iter()
                .map(|e| instruction_item(HOME, e))
                .collect::<String>()
        ));
    }
    if !skills.is_empty() {
        body.push_str(&format!(
            "<section><h2>Skills</h2><ul class=\"agent-grid\">{}</ul></section>",
            skills
                .iter()
                .map(|e| skill_card(HOME, e))
                .collect::<String>()
        ));
    }
    let connected: String = agents
        .entries
        .iter()
        .filter(|e| !e.references.is_empty())
        .map(|e| {
            format!(
                "<li>{} → {}</li>",
                link(HOME, &e.route, &e.title),
                e.references
                    .iter()
                    .map(|r| link(HOME, &r.route, &r.title))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
        .collect();
    if !connected.is_empty() {
        body.push_str(&format!("<section><h2>Connected knowledge</h2><p class=\"note\">What each artifact references. Every relationship is also in the <a href=\"{}\">graph</a>.</p><ul>{connected}</ul></section>", escape(&relative_url(HOME, "/__entwine/graph/"))));
    }
    files.push(shell(HOME, "Agent knowledge", &body, "Overview"));

    // Instructions
    let route = "/__entwine/agents/instructions/";
    let mut body = String::from("<p class=\"note\">Instruction files grouped by the directory they live in. Each applies, structurally, to that directory and below.</p>");
    for scope in &scopes {
        body.push_str(&format!(
            "<section><h2>{}</h2><ul>{}</ul></section>",
            escape(scope_label(scope)),
            instructions
                .iter()
                .filter(|e| e.details.scope.as_deref() == Some(scope))
                .map(|e| instruction_item(route, e))
                .collect::<String>()
        ));
    }
    if instructions.is_empty() {
        body.push_str("<p class=\"missing\">No instruction files were discovered.</p>");
    }
    files.push(shell(route, "Agent instructions", &body, "Instructions"));

    // Skills
    let route = "/__entwine/agents/skills/";
    let body = if skills.is_empty() {
        "<p class=\"missing\">No skills were discovered.</p>".to_string()
    } else {
        format!(
            "<p class=\"note\">Each skill is a folder with a <code>SKILL.md</code> manifest and optional supporting resources. Resources are listed, never run or copied.</p><ul class=\"agent-grid\">{}</ul>",
            skills.iter().map(|e| skill_card(route, e)).collect::<String>()
        )
    };
    files.push(shell(route, "Skills", &body, "Skills"));

    // Scopes
    let route = "/__entwine/agents/scopes/";
    let mut body = String::from("<p class=\"note\">Scopes show where instruction files sit in the directory tree. Files in an ancestor scope are structurally relevant to its descendants; how any given tool combines or prioritizes them is up to that tool, and Entwine does not simulate it.</p><div class=\"scope-tree\"><ul>");
    for scope in &scopes {
        let depth = scopes
            .iter()
            .filter(|other| {
                *other != scope && (other.is_empty() || scope.starts_with(&format!("{other}/")))
            })
            .count();
        body.push_str(&format!(
            "<li style=\"margin-left:{}rem\"><h2>{}</h2><ul>{}</ul></li>",
            depth as f64 * 1.25,
            escape(scope_label(scope)),
            instructions
                .iter()
                .filter(|e| e.details.scope.as_deref() == Some(scope))
                .map(|e| instruction_item(route, e))
                .collect::<String>()
        ));
    }
    body.push_str("</ul></div>");
    if scopes.is_empty() {
        body = "<p class=\"missing\">No instruction scopes were discovered.</p>".into();
    }
    files.push(shell(route, "Scopes", &body, "Scopes"));

    // One page per artifact
    for entry in &agents.entries {
        let route = entry.route.as_str();
        let details = &entry.details;
        let mut meta = format!(
            "<dt>Kind</dt><dd>{}</dd><dt>Convention</dt><dd><code>{}</code></dd>",
            entry.kind.label(),
            details.convention.file_name()
        );
        let source = match &entry.source_url {
            Some(url) => format!(
                "<a href=\"{}\"><code>{}</code></a>",
                escape(url),
                escape(&details.path)
            ),
            None => format!("<code>{}</code>", escape(&details.path)),
        };
        meta.push_str(&format!("<dt>Source</dt><dd>{source}</dd>"));
        if let Some(scope) = &details.scope {
            meta.push_str(&format!(
                "<dt>Scope</dt><dd>{}</dd>",
                escape(scope_label(scope))
            ));
        }
        if let Some(name) = &details.name {
            meta.push_str(&format!("<dt>Name</dt><dd>{}</dd>", escape(name)));
        }
        if let Some(description) = &details.description {
            meta.push_str(&format!(
                "<dt>Description</dt><dd>{}</dd>",
                escape(description)
            ));
        }
        if let Some(directory) = &details.skill_directory {
            meta.push_str(&format!(
                "<dt>Skill folder</dt><dd><code>{}</code></dd>",
                escape(if directory.is_empty() { "." } else { directory })
            ));
        }
        let graph = format!(
            "{}?focus={}",
            relative_url(route, "/__entwine/graph/"),
            encode_component(&entry.id.0)
        );
        meta.push_str(&format!(
            "<dt>Graph</dt><dd><a href=\"{}\">Show in graph</a></dd>",
            escape(&graph)
        ));
        let mut extra = String::new();
        if details.skill_directory.is_some() {
            let items: String = entry
                .resource_links
                .iter()
                .map(|(path, url)| match url {
                    Some(url) => format!(
                        "<li><a href=\"{}\"><code>{}</code></a></li>",
                        escape(url),
                        escape(path)
                    ),
                    None => format!("<li><code>{}</code></li>", escape(path)),
                })
                .collect();
            extra.push_str(&if items.is_empty() {
                "<section><h2>Resources</h2><p class=\"missing\">No colocated files.</p></section>".into()
            } else {
                format!("<section><h2>Resources</h2><p class=\"note\">Files colocated with this skill. They are not published or executed.</p><ul>{items}</ul></section>")
            });
        }
        if !entry.references.is_empty() {
            extra.push_str(&format!(
                "<section><h2>References</h2><ul>{}</ul></section>",
                links(route, &entry.references)
            ));
        }
        if !entry.backlinks.is_empty() {
            extra.push_str(&format!(
                "<section><h2>Referenced by</h2><ul>{}</ul></section>",
                links(route, &entry.backlinks)
            ));
        }
        let body = format!(
            "<dl class=\"agent-meta\">{meta}</dl>{extra}<article class=\"agent-body\">{}</article>",
            entry.html
        );
        let active = if entry.kind == ArtifactKind::Skill {
            "Skills"
        } else {
            "Instructions"
        };
        files.push(shell(route, &entry.title, &body, active));
    }
    files
}
