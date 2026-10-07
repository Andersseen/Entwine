//! Generated Agents views: what agent-facing knowledge exists, where it lives, what scope it
//! covers, and how it connects to documentation. Rendered only when publication is enabled.
use crate::render::{escape, frame, StaticFile};
use crate::resolve::{encode_component, relative_url};
use entwine_core::*;
use std::collections::{BTreeMap, BTreeSet};

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
fn plural(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}
fn kind_name(kind: ArtifactKind) -> &'static str {
    match kind {
        ArtifactKind::Documentation => "Documentation",
        ArtifactKind::AgentInstructions => "Instructions",
        ArtifactKind::Skill => "Skill",
    }
}
/// A link plus what it points at and where its source lives.
fn artifact_item(from: &str, target: &AgentLink) -> String {
    format!(
        "<li>{} <span class=\"role-tag\">{}</span> <code>{}</code></li>",
        link(from, &target.route, &target.title),
        kind_name(target.kind),
        escape(&target.path)
    )
}
fn shorten(text: &str, max: usize) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= max {
        flat
    } else {
        flat.chars()
            .take(max - 1)
            .collect::<String>()
            .trim_end()
            .to_string()
            + "…"
    }
}

fn instruction_item(from: &str, entry: &AgentEntry) -> String {
    format!(
        "<li>{} <span class=\"role-tag\">{}</span> <code>{}</code></li>",
        link(from, &entry.route, &entry.title),
        entry.details.convention.file_name(),
        escape(&entry.details.path)
    )
}

/// The directory that holds a skill's folder (for example `.agents/skills`).
fn skill_location(entry: &AgentEntry) -> &str {
    entry
        .details
        .skill_directory
        .as_deref()
        .map_or("", |directory| {
            directory.rsplit_once('/').map_or("", |(parent, _)| parent)
        })
}

fn skill_row(from: &str, entry: &AgentEntry) -> String {
    let details = &entry.details;
    let mut meta = Vec::new();
    if !details.resources.is_empty() {
        meta.push(plural(details.resources.len(), "resource", "resources"));
    }
    if !entry.references.is_empty() {
        meta.push(plural(entry.references.len(), "reference", "references"));
    }
    if !entry.exposures.is_empty() {
        meta.push(format!(
            "exposed through {}",
            entry
                .exposures
                .iter()
                .map(|e| format!(
                    "<a href=\"{}\"><code>{}</code></a>",
                    escape(&relative_url(from, e.route.as_str())),
                    escape(e.path.trim_end_matches("/SKILL.md"))
                ))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    format!(
        "<li class=\"skill-row\"><div class=\"skill-head\">{} <code>{}</code></div>{}{}</li>",
        link(from, &entry.route, &entry.title),
        escape(details.skill_directory.as_deref().unwrap_or("")),
        details
            .description
            .as_deref()
            .map(|d| format!("<p class=\"skill-desc\">{}</p>", escape(&shorten(d, 220))))
            .unwrap_or_default(),
        if meta.is_empty() {
            String::new()
        } else {
            format!("<div class=\"skill-meta\">{}</div>", meta.join(" · "))
        }
    )
}

/// Canonical (and standalone) skills grouped by the directory their folders live in.
fn skill_groups(from: &str, skills: &[&AgentEntry]) -> String {
    let mut groups: BTreeMap<&str, Vec<&AgentEntry>> = BTreeMap::new();
    for skill in skills {
        groups.entry(skill_location(skill)).or_default().push(skill);
    }
    groups
        .into_iter()
        .map(|(location, entries)| {
            format!(
                "<section class=\"skill-group\"><h3><code>{}</code> <span class=\"count\">{}</span></h3><ul class=\"skill-list\">{}</ul></section>",
                escape(if location.is_empty() { "." } else { location }),
                entries.len(),
                entries.iter().map(|e| skill_row(from, e)).collect::<String>()
            )
        })
        .collect()
}

/// Nested scopes: a scope sits under its closest enclosing scope. Purely structural.
fn scope_tree(from: &str, instructions: &[&AgentEntry]) -> String {
    let scopes: BTreeSet<&str> = instructions
        .iter()
        .filter_map(|e| e.details.scope.as_deref())
        .collect();
    let enclosing = |scope: &str| -> Option<&str> {
        scopes
            .iter()
            .filter(|other| {
                **other != scope && (other.is_empty() || scope.starts_with(&format!("{other}/")))
            })
            .max_by_key(|other| other.len())
            .copied()
    };
    let mut children: BTreeMap<Option<&str>, Vec<&str>> = BTreeMap::new();
    for scope in &scopes {
        children.entry(enclosing(scope)).or_default().push(scope);
    }
    fn emit(
        parent: Option<&str>,
        children: &BTreeMap<Option<&str>, Vec<&str>>,
        from: &str,
        instructions: &[&AgentEntry],
    ) -> String {
        let Some(list) = children.get(&parent) else {
            return String::new();
        };
        let items: String = list
            .iter()
            .map(|scope| {
                let files: String = instructions
                    .iter()
                    .filter(|e| e.details.scope.as_deref() == Some(*scope))
                    .map(|e| {
                        format!(
                            "<a class=\"scope-file\" href=\"{}\" title=\"{}\">{}</a>",
                            escape(&relative_url(from, e.route.as_str())),
                            escape(&e.details.path),
                            e.details.convention.file_name()
                        )
                    })
                    .collect();
                format!(
                    "<li><div class=\"scope-head\"><span class=\"scope-name\">{}</span>{files}</div>{}</li>",
                    escape(scope_label(scope)),
                    emit(Some(scope), children, from, instructions)
                )
            })
            .collect();
        format!("<ul>{items}</ul>")
    }
    format!(
        "<div class=\"scope-tree\">{}</div>",
        emit(None, &children, from, instructions)
    )
}

fn stat(label: &str, value: usize) -> String {
    format!("<div><dt>{label}</dt><dd>{value}</dd></div>")
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
    let all_skills: Vec<&AgentEntry> = agents
        .entries
        .iter()
        .filter(|e| e.kind == ArtifactKind::Skill)
        .collect();
    // A manifest that only exposes another skill is not a separate skill.
    let skills: Vec<&AgentEntry> = all_skills
        .iter()
        .copied()
        .filter(|e| e.canonical.is_none())
        .collect();
    let exposure_count = all_skills.len() - skills.len();
    let scopes: BTreeSet<&str> = instructions
        .iter()
        .filter_map(|e| e.details.scope.as_deref())
        .collect();
    let resource_count: usize = all_skills.iter().map(|e| e.details.resources.len()).sum();
    let mut files = Vec::new();

    // Overview
    let mut body = format!("<p class=\"note\">Agent-facing knowledge found in this repository. Documentation lives in the <a href=\"{}\">main site</a>; this section keeps instructions and skills structured instead of mixing them in.</p>",
        escape(&relative_url(HOME, "/")));
    body.push_str(&format!(
        "<dl class=\"agent-stats\">{}{}{}{}{}</dl>",
        stat("Instruction files", instructions.len()),
        stat("Scopes", scopes.len()),
        stat("Skills", skills.len()),
        if exposure_count > 0 {
            stat("Skill exposures", exposure_count)
        } else {
            String::new()
        },
        stat("Skill resources", resource_count),
    ));
    if exposure_count > 0 {
        body.push_str(&format!(
            "<p class=\"note\">{} discovered <code>SKILL.md</code> {} only {} a skill defined elsewhere (same declared name, a link to the canonical manifest, no files of their own). {} listed under the skill {} {}, not counted as separate skills.</p>",
            exposure_count,
            if exposure_count == 1 { "file" } else { "files" },
            if exposure_count == 1 { "exposes" } else { "expose" },
            if exposure_count == 1 { "It is" } else { "They are" },
            if exposure_count == 1 { "it" } else { "they" },
            if exposure_count == 1 { "exposes" } else { "expose" },
        ));
    }
    if !instructions.is_empty() {
        body.push_str(&format!(
            "<section><h2>Instructions</h2><p class=\"note\">Where instruction files sit in the directory tree. Scope is structural; how a tool combines these files is up to that tool.</p>{}</section>",
            scope_tree(HOME, &instructions)
        ));
    }
    if !skills.is_empty() {
        body.push_str(&format!(
            "<section><h2>Skills</h2>{}</section>",
            skill_groups(HOME, &skills)
        ));
    }
    let docs_into_agents: Vec<(&AgentLink, Vec<&AgentEntry>)> = {
        let mut map: BTreeMap<&Route, (&AgentLink, Vec<&AgentEntry>)> = BTreeMap::new();
        for entry in &agents.entries {
            for back in entry
                .backlinks
                .iter()
                .filter(|b| b.kind == ArtifactKind::Documentation)
            {
                map.entry(&back.route)
                    .or_insert((back, Vec::new()))
                    .1
                    .push(entry);
            }
        }
        map.into_values().collect()
    };
    let agents_into_docs: Vec<(&AgentEntry, Vec<&AgentLink>)> = agents
        .entries
        .iter()
        .filter(|e| e.canonical.is_none())
        .map(|e| {
            (
                e,
                e.references
                    .iter()
                    .filter(|r| r.kind == ArtifactKind::Documentation)
                    .collect::<Vec<_>>(),
            )
        })
        .filter(|(_, docs)| !docs.is_empty())
        .collect();
    if !docs_into_agents.is_empty() || !agents_into_docs.is_empty() {
        let mut section = String::from("<section><h2>Connected to documentation</h2>");
        if !agents_into_docs.is_empty() {
            section.push_str("<h3>Agent files that reference documentation</h3><ul>");
            for (entry, docs) in &agents_into_docs {
                section.push_str(&format!(
                    "<li>{} → {}</li>",
                    link(HOME, &entry.route, &entry.title),
                    docs.iter()
                        .map(|d| link(HOME, &d.route, &d.title))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            section.push_str("</ul>");
        }
        if !docs_into_agents.is_empty() {
            section.push_str("<h3>Documentation that references agent files</h3><ul>");
            for (doc, entries) in &docs_into_agents {
                section.push_str(&format!(
                    "<li>{} → {}</li>",
                    link(HOME, &doc.route, &doc.title),
                    entries
                        .iter()
                        .map(|e| link(HOME, &e.route, &e.title))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            section.push_str("</ul>");
        }
        section.push_str(&format!(
            "<p class=\"note\">Every relationship is also in the <a href=\"{}\">graph</a>.</p></section>",
            escape(&relative_url(HOME, "/__entwine/graph/"))
        ));
        body.push_str(&section);
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
            "<p class=\"note\">Each skill is a folder with a <code>SKILL.md</code> manifest and optional supporting resources, grouped by where the folders live. Resources are listed, never run or copied.</p>{}",
            skill_groups(route, &skills)
        )
    };
    files.push(shell(route, "Skills", &body, "Skills"));

    // Scopes
    let route = "/__entwine/agents/scopes/";
    let body = if scopes.is_empty() {
        "<p class=\"missing\">No instruction scopes were discovered.</p>".into()
    } else {
        format!(
            "<p class=\"note\">Scopes show where instruction files sit in the directory tree. Files in an ancestor scope are structurally relevant to its descendants. Which of them a given tool loads, and in what order, is that tool's decision; Entwine does not simulate it.</p>{}",
            scope_tree(route, &instructions)
        )
    };
    files.push(shell(route, "Scopes", &body, "Scopes"));

    // One page per artifact
    for entry in &agents.entries {
        let route = entry.route.as_str();
        let details = &entry.details;
        let mut intro = String::new();
        if let Some(canonical) = &entry.canonical {
            intro.push_str(&format!(
                "<p class=\"callout\">This manifest only <strong>exposes</strong> a skill defined elsewhere. Canonical source: {} <code>{}</code>. Entwine treats it as an exposure because it declares the same name, links to the canonical <code>SKILL.md</code>, and has no files of its own.</p>",
                link(route, &canonical.route, &canonical.title),
                escape(&canonical.path)
            ));
        }
        let mut meta = format!(
            "<dt>Kind</dt><dd>{}{}</dd><dt>Convention</dt><dd><code>{}</code></dd>",
            entry.kind.label(),
            if entry.canonical.is_some() {
                " (exposure)"
            } else {
                ""
            },
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
        meta.push_str(&format!(
            "<dt>{}</dt><dd>{source}</dd>",
            if entry.canonical.is_some() {
                "This file"
            } else if entry.kind == ArtifactKind::Skill {
                "Canonical source"
            } else {
                "Source"
            }
        ));
        if let Some(canonical) = &entry.canonical {
            meta.push_str(&format!(
                "<dt>Canonical source</dt><dd>{} <code>{}</code></dd>",
                link(route, &canonical.route, &canonical.title),
                escape(&canonical.path)
            ));
        }
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
            let location = skill_location(entry);
            meta.push_str(&format!(
                "<dt>Skill folder</dt><dd><code>{}</code></dd><dt>Location</dt><dd><code>{}</code></dd>",
                escape(if directory.is_empty() { "." } else { directory }),
                escape(if location.is_empty() { "." } else { location })
            ));
        }
        if !entry.exposures.is_empty() {
            meta.push_str(&format!(
                "<dt>Exposed through</dt><dd><ul class=\"plain\">{}</ul></dd>",
                entry
                    .exposures
                    .iter()
                    .map(|e| format!(
                        "<li>{} <code>{}</code></li>",
                        link(route, &e.route, &e.title),
                        escape(&e.path)
                    ))
                    .collect::<String>()
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
        if details.skill_directory.is_some() && entry.canonical.is_none() {
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
        let exposure_routes: BTreeSet<&Route> = entry.exposures.iter().map(|e| &e.route).collect();
        let references: Vec<&AgentLink> = entry
            .references
            .iter()
            .filter(|r| entry.canonical.as_ref().is_none_or(|c| c.route != r.route))
            .collect();
        if !references.is_empty() {
            extra.push_str(&format!(
                "<section><h2>References</h2><ul>{}</ul></section>",
                references
                    .iter()
                    .map(|r| artifact_item(route, r))
                    .collect::<String>()
            ));
        }
        let referenced_by: Vec<&AgentLink> = entry
            .backlinks
            .iter()
            .filter(|b| !exposure_routes.contains(&b.route))
            .collect();
        for (kind, heading) in [
            (ArtifactKind::Documentation, "Referenced by documentation"),
            (
                ArtifactKind::AgentInstructions,
                "Referenced by instructions",
            ),
            (ArtifactKind::Skill, "Referenced by skills"),
        ] {
            let group: String = referenced_by
                .iter()
                .filter(|b| b.kind == kind)
                .map(|b| artifact_item(route, b))
                .collect();
            if !group.is_empty() {
                extra.push_str(&format!(
                    "<section><h2>{heading}</h2><ul>{group}</ul></section>"
                ));
            }
        }
        let body = format!(
            "{intro}<dl class=\"agent-meta\">{meta}</dl>{extra}<article class=\"agent-body\">{}</article>",
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
