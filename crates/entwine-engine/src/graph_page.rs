//! The graph page: a fully rendered static SVG, an embedded JSON description of the same
//! graph, and a complete text index. `graph.js` progressively enhances the SVG; nothing
//! here needs JavaScript to be readable or complete.
use crate::graph_layout::{edge_path, file_key, layout, radius, Layout, Point};
use crate::render::{escape, frame_with, StaticFile};
use crate::resolve::relative_url;
use entwine_core::*;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

const ROUTE: &str = "/__entwine/graph/";
/// Above this size the text index leads and the visual graph starts collapsed.
const LARGE_NODES: usize = 150;
const LARGE_EDGES: usize = 600;

/// The enhancement script, shipped as a separate static file.
pub fn render_graph_script() -> StaticFile {
    StaticFile {
        path: "__entwine/graph.js".into(),
        contents: include_bytes!("graph.js").to_vec(),
    }
}

fn glyph(node: &GraphNode) -> &'static str {
    match node.artifact {
        ArtifactKind::AgentInstructions => "In",
        ArtifactKind::Skill => "Sk",
        ArtifactKind::Documentation => match node.role {
            KnowledgeRole::Project => "P",
            KnowledgeRole::Architecture => "A",
            KnowledgeRole::State => "St",
            KnowledgeRole::Roadmap => "R",
            KnowledgeRole::Decision => "D",
            KnowledgeRole::Spec => "Sp",
            KnowledgeRole::Other => "·",
        },
    }
}

/// Full accessible name: the label plus what kind of thing it is.
fn node_name(node: &GraphNode) -> String {
    match node.artifact {
        ArtifactKind::Documentation if node.role == KnowledgeRole::Other => node.label.clone(),
        ArtifactKind::Documentation => format!("{} ({})", node.label, node.role.label()),
        kind => format!("{} ({})", node.label, kind.label()),
    }
}

fn shape(artifact: Option<ArtifactKind>, r: f64) -> String {
    match artifact {
        Some(ArtifactKind::AgentInstructions) => format!(
            "<rect x=\"{0}\" y=\"{0}\" width=\"{1}\" height=\"{1}\" rx=\"5\"/>",
            -r,
            r * 2.0
        ),
        Some(ArtifactKind::Skill) => {
            let d = r * 1.3;
            format!("<polygon points=\"0,{} {d},0 0,{d} {},0\"/>", -d, -d)
        }
        Some(ArtifactKind::Documentation) => format!("<circle r=\"{r}\"/>"),
        None => "<rect x=\"-8\" y=\"-8\" width=\"16\" height=\"16\" rx=\"2\"/>".into(),
    }
}

/// Keep the end of a path, which is the part that tells paths apart.
fn truncate_start(value: &str, max: usize) -> String {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= max {
        value.to_string()
    } else {
        format!(
            "…{}",
            chars[chars.len() - (max - 1)..].iter().collect::<String>()
        )
    }
}

fn truncate(label: &str, max: usize) -> String {
    let chars: Vec<char> = label.trim().chars().collect();
    if chars.len() <= max {
        chars.into_iter().collect()
    } else {
        chars[..max - 1].iter().collect::<String>() + "…"
    }
}

fn is_true(value: &bool) -> bool {
    *value
}

#[derive(Serialize)]
struct JsonNode<'a> {
    id: &'a str,
    label: &'a str,
    href: Option<String>,
    kind: &'a str,
    role: &'a str,
    role_label: &'a str,
    kind_label: &'a str,
    path: &'a str,
    scope: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    location: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    qualifier: Option<String>,
    /// False when the node has no visible relationship (laid out in the unlinked block).
    #[serde(skip_serializing_if = "is_true")]
    linked: bool,
    /// The canonical skill this node is an exposure of (hidden unless exposures are shown).
    #[serde(skip_serializing_if = "Option::is_none")]
    exposure_of: Option<&'a str>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    exposed_through: Vec<&'a str>,
    x: f64,
    y: f64,
    r: f64,
    out: usize,
    #[serde(rename = "in")]
    incoming: usize,
    refs: usize,
    referenced_by: Vec<&'a str>,
}
#[derive(Serialize)]
struct JsonGraph<'a> {
    width: f64,
    height: f64,
    nodes: Vec<JsonNode<'a>>,
    edges: Vec<(&'a str, &'a str)>,
}

fn json_script(data: &JsonGraph) -> String {
    let json = serde_json::to_string(data).unwrap_or_else(|_| "{}".into());
    // Nothing in the payload may close the script element or start a comment.
    json.replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029")
}

fn legend(graph: &GraphModel) -> String {
    let has = |kind: ArtifactKind| graph.nodes.iter().any(|n| n.artifact == kind);
    let mut items = Vec::new();
    let icon = |artifact: Option<ArtifactKind>, class: &str, glyph: &str| {
        format!(
            "<svg class=\"legend-icon\" viewBox=\"-20 -20 40 40\" width=\"28\" height=\"28\" aria-hidden=\"true\"><g class=\"graph-node {class}\"><g class=\"shape\">{}<text class=\"glyph\" text-anchor=\"middle\" dy=\"0.35em\">{glyph}</text></g></g></svg>",
            shape(artifact, 14.0)
        )
    };
    if has(ArtifactKind::Documentation) {
        for role in RECOMMENDED_ROLES
            .iter()
            .chain(std::iter::once(&KnowledgeRole::Other))
        {
            if graph
                .nodes
                .iter()
                .any(|n| n.artifact == ArtifactKind::Documentation && n.role == *role)
            {
                let g = glyph(&GraphNode {
                    id: DocumentId(String::new()),
                    label: String::new(),
                    route: Route::home(),
                    metadata: DocumentMetadata::default(),
                    role: *role,
                    artifact: ArtifactKind::Documentation,
                    path: String::new(),
                    scope: None,
                    location: None,
                    exposure_of: None,
                    exposures: 0,
                });
                items.push(format!(
                    "<li>{} {}</li>",
                    icon(
                        Some(ArtifactKind::Documentation),
                        &format!("kind-documentation role-{}", role.as_str()),
                        g
                    ),
                    role.label()
                ));
            }
        }
    }
    if has(ArtifactKind::AgentInstructions) {
        items.push(format!(
            "<li>{} Agent instructions</li>",
            icon(
                Some(ArtifactKind::AgentInstructions),
                "kind-agent_instructions",
                "In"
            )
        ));
    }
    if has(ArtifactKind::Skill) {
        items.push(format!(
            "<li>{} Skill</li>",
            icon(Some(ArtifactKind::Skill), "kind-skill", "Sk")
        ));
    }
    if graph.nodes.iter().any(|n| n.exposure_of.is_some()) {
        items.push(format!(
            "<li>{} Skill exposure (hidden by default)</li>",
            icon(Some(ArtifactKind::Skill), "kind-skill is-exposure", "Sk")
        ));
    }
    if !graph.files.is_empty() {
        items.push(format!(
            "<li>{} Repository file (optional)</li>",
            icon(None, "kind-repository_file", "")
        ));
    }
    format!(
        "<ul class=\"graph-legend\" aria-label=\"Legend: shape shows the kind of artifact, letters show the role\">{}</ul>",
        items.join("")
    )
}

/// Deterministic SVG with linked nodes, directional edges, a text index, and a hook for
/// the interactive layer.
pub fn render_graph(graph: &GraphModel) -> StaticFile {
    let layout: Layout = layout(graph, true);
    let degree_of = |key: &str| layout.degrees.get(key).copied().unwrap_or(0);
    let point = |key: &str| -> Option<Point> { layout.positions.get(key).copied() };
    let exposures: BTreeSet<&DocumentId> = graph
        .nodes
        .iter()
        .filter(|n| n.exposure_of.is_some())
        .map(|n| &n.id)
        .collect();
    // Counts describe what is visible by default: edges to exposures are listed on the
    // canonical node as "exposed through", not as links.
    let mut incoming: BTreeMap<&DocumentId, usize> = BTreeMap::new();
    let mut outgoing: BTreeMap<&DocumentId, usize> = BTreeMap::new();
    for edge in &graph.edges {
        if exposures.contains(&edge.source) || exposures.contains(&edge.target) {
            continue;
        }
        *outgoing.entry(&edge.source).or_default() += 1;
        *incoming.entry(&edge.target).or_default() += 1;
    }
    let mut label_counts: BTreeMap<&str, usize> = BTreeMap::new();
    for node in graph.nodes.iter().filter(|n| n.exposure_of.is_none()) {
        *label_counts.entry(node.label.as_str()).or_default() += 1;
    }
    let exposed_through: BTreeMap<&DocumentId, Vec<&GraphNode>> = {
        let mut map: BTreeMap<&DocumentId, Vec<&GraphNode>> = BTreeMap::new();
        for node in &graph.nodes {
            if let Some(canonical) = &node.exposure_of {
                map.entry(canonical).or_default().push(node);
            }
        }
        map
    };
    // A second line tells apart nodes whose titles collide; exposures always carry one.
    let qualifier = |node: &GraphNode| -> Option<String> {
        let ambiguous = node.exposure_of.is_some()
            || label_counts.get(node.label.as_str()).copied().unwrap_or(0) > 1;
        let place = node
            .location
            .as_deref()
            .or_else(|| node.path.rsplit_once('/').map(|(dir, _)| dir))?;
        ambiguous.then(|| truncate_start(if place.is_empty() { "." } else { place }, 28))
    };
    let (width, height) = (layout.width, layout.height);
    let kinds = graph
        .nodes
        .iter()
        .filter(|n| n.artifact.is_agent_facing())
        .count();
    let mut svg = format!("<svg class=\"graph\" xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\" role=\"group\" aria-labelledby=\"graph-title graph-description\"><title id=\"graph-title\">Project knowledge graph</title><desc id=\"graph-description\">{} linked nodes and {} directed references. Nodes are clustered by artifact kind and knowledge role. A complete text list of every node and relationship follows the graph.</desc><defs><marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"5\" markerHeight=\"5\" orient=\"auto\"><path d=\"M 0 0 L 10 5 L 0 10 z\"/></marker></defs><g class=\"viewport\"><g class=\"edges\">", graph.nodes.len(), graph.edges.len());
    let node_by_id: BTreeMap<&DocumentId, &GraphNode> =
        graph.nodes.iter().map(|n| (&n.id, n)).collect();
    for edge in &graph.edges {
        if let (Some(s), Some(t)) = (point(&edge.source.0), point(&edge.target.0)) {
            let path = edge_path(
                s,
                t,
                radius(degree_of(&edge.source.0)),
                radius(degree_of(&edge.target.0)),
            );
            let name = |id: &DocumentId| {
                node_by_id
                    .get(id)
                    .map_or_else(|| id.0.clone(), |n| n.label.clone())
            };
            let hidden = if exposures.contains(&edge.source) || exposures.contains(&edge.target) {
                " is-off"
            } else {
                ""
            };
            svg.push_str(&format!("<path class=\"edge{hidden}\" data-source=\"{}\" data-target=\"{}\" d=\"{path}\" marker-end=\"url(#arrow)\"><title>{} references {}</title></path>", escape(&edge.source.0), escape(&edge.target.0), escape(&name(&edge.source)), escape(&name(&edge.target))));
        }
    }
    for file in &graph.files {
        let key = file_key(&file.path);
        let Some(target) = point(&key) else { continue };
        for source in &file.referenced_by {
            if let Some(origin) = point(&source.0) {
                let path = edge_path(origin, target, radius(degree_of(&source.0)), 9.0);
                svg.push_str(&format!("<path class=\"edge edge-file is-off\" data-source=\"{}\" data-target=\"{}\" d=\"{path}\" marker-end=\"url(#arrow)\"/>", escape(&source.0), escape(&key)));
            }
        }
    }
    svg.push_str("</g><g class=\"nodes\">");
    let route_href = |route: &Route| relative_url(ROUTE, route.as_str());
    let mut json_nodes = Vec::new();
    let refs_of = |id: &DocumentId| {
        graph
            .files
            .iter()
            .filter(|f| f.referenced_by.contains(id))
            .count()
    };
    for node in &graph.nodes {
        let Some(p) = point(&node.id.0) else { continue };
        let r = radius(degree_of(&node.id.0));
        let name = node_name(node);
        let href = route_href(&node.route);
        let exposure = node.exposure_of.is_some();
        let qualifier = qualifier(node);
        let classes = format!(
            "graph-node kind-{} role-{}{}",
            node.artifact.as_str(),
            node.role.as_str(),
            if exposure { " is-exposure is-off" } else { "" }
        );
        let qualifier_text = qualifier.as_deref().map_or_else(String::new, |q| {
            format!(
                "<text class=\"node-qualifier\" text-anchor=\"middle\" y=\"{}\">{}</text>",
                r + 31.0,
                escape(q)
            )
        });
        svg.push_str(&format!("<a class=\"{classes}\" href=\"{}\" data-node=\"{}\" data-kind=\"{}\" data-role=\"{}\" aria-label=\"{}\" transform=\"translate({} {})\"><title>{}</title><g class=\"shape\">{}<text class=\"glyph\" text-anchor=\"middle\" dy=\"0.35em\">{}</text></g><text class=\"node-label\" text-anchor=\"middle\" y=\"{}\">{}</text>{qualifier_text}</a>",
            escape(&href), escape(&node.id.0), node.artifact.as_str(), node.role.as_str(), escape(&name), p.x, p.y, escape(&name),
            shape(Some(node.artifact), r), glyph(node), r + 17.0, escape(&truncate(&node.label, 24))));
        json_nodes.push(JsonNode {
            id: &node.id.0,
            label: &node.label,
            href: Some(href),
            kind: node.artifact.as_str(),
            role: node.role.as_str(),
            role_label: node.role.label(),
            kind_label: node.artifact.label(),
            path: &node.path,
            scope: node.scope.as_deref(),
            location: node.location.as_deref(),
            qualifier,
            linked: !layout.unlinked.contains(&node.id.0),
            exposure_of: node.exposure_of.as_ref().map(|id| id.0.as_str()),
            exposed_through: exposed_through
                .get(&node.id)
                .map(|list| list.iter().map(|n| n.id.0.as_str()).collect())
                .unwrap_or_default(),
            x: p.x,
            y: p.y,
            r,
            out: outgoing.get(&node.id).copied().unwrap_or(0),
            incoming: incoming.get(&node.id).copied().unwrap_or(0),
            refs: refs_of(&node.id),
            referenced_by: Vec::new(),
        });
    }
    let file_keys: Vec<String> = graph.files.iter().map(|f| file_key(&f.path)).collect();
    for (file, key) in graph.files.iter().zip(&file_keys) {
        let Some(p) = point(key) else { continue };
        svg.push_str(&format!("<g class=\"graph-node kind-repository_file is-off\" data-node=\"{}\" transform=\"translate({} {})\"><title>{}</title><g class=\"shape\">{}</g><text class=\"node-label\" text-anchor=\"middle\" y=\"26\">{}</text></g>",
            escape(key), p.x, p.y, escape(&file.path), shape(None, 8.0), escape(&truncate(file.path.rsplit('/').next().unwrap_or(&file.path), 24))));
        json_nodes.push(JsonNode {
            id: key,
            label: file.path.rsplit('/').next().unwrap_or(&file.path),
            href: None,
            kind: "repository_file",
            role: "other",
            role_label: "",
            kind_label: "Repository file",
            path: &file.path,
            scope: None,
            location: None,
            qualifier: None,
            linked: true,
            exposure_of: None,
            exposed_through: Vec::new(),
            x: p.x,
            y: p.y,
            r: 9.0,
            out: 0,
            incoming: file.referenced_by.len(),
            refs: 0,
            referenced_by: file.referenced_by.iter().map(|d| d.0.as_str()).collect(),
        });
    }
    svg.push_str("</g></g></svg>");
    let data = json_script(&JsonGraph {
        width,
        height,
        nodes: json_nodes,
        edges: graph
            .edges
            .iter()
            .map(|e| (e.source.0.as_str(), e.target.0.as_str()))
            .collect(),
    });

    // Complete textual companion: authoritative regardless of graph density or JavaScript.
    let mut ordered: Vec<_> = graph.nodes.iter().collect();
    ordered.sort_by(|a, b| {
        (a.artifact, a.role.rank(), &a.id).cmp(&(b.artifact, b.role.rank(), &b.id))
    });
    let entry = |n: &GraphNode| {
        let nested = exposed_through
            .get(&n.id)
            .map(|list| {
                format!(
                    "<ul class=\"exposure-list\">{}</ul>",
                    list.iter()
                        .map(|e| format!(
                            "<li><span class=\"role-tag\">exposed through</span> <a href=\"{}\"><code>{}</code></a></li>",
                            escape(&route_href(&e.route)),
                            escape(&e.path)
                        ))
                        .collect::<String>()
                )
            })
            .unwrap_or_default();
        format!(
            "<li><a href=\"{}\">{}</a> <code>{}</code>{}{nested}</li>",
            escape(&route_href(&n.route)),
            escape(&n.label),
            escape(&n.path),
            match (&n.artifact, &n.scope) {
                (ArtifactKind::AgentInstructions, Some(scope)) => format!(
                    " <span class=\"role-tag\">scope: {}</span>",
                    escape(if scope.is_empty() {
                        "project root"
                    } else {
                        scope
                    })
                ),
                _ => String::new(),
            }
        )
    };
    let mut groups = String::new();
    for role in RECOMMENDED_ROLES
        .iter()
        .chain(std::iter::once(&KnowledgeRole::Other))
    {
        let items = ordered
            .iter()
            .filter(|n| n.artifact == ArtifactKind::Documentation && n.role == *role)
            .map(|n| entry(n))
            .collect::<String>();
        if !items.is_empty() {
            groups.push_str(&format!(
                "<section><h3>{}</h3><ul>{items}</ul></section>",
                role.label()
            ));
        }
    }
    for kind in [ArtifactKind::AgentInstructions, ArtifactKind::Skill] {
        let items = ordered
            .iter()
            .filter(|n| n.artifact == kind && n.exposure_of.is_none())
            .map(|n| entry(n))
            .collect::<String>();
        if !items.is_empty() {
            groups.push_str(&format!(
                "<section><h3>{}</h3><ul>{items}</ul></section>",
                if kind == ArtifactKind::Skill {
                    "Skills"
                } else {
                    "Agent instructions"
                }
            ));
        }
    }
    let relation_tag = |n: &GraphNode| match n.artifact {
        ArtifactKind::Documentation => String::new(),
        _ if n.exposure_of.is_some() => format!(
            " <span class=\"role-tag\">{} exposure</span> <code>{}</code>",
            n.artifact.label(),
            escape(&n.path)
        ),
        kind => {
            let ambiguous = label_counts.get(n.label.as_str()).copied().unwrap_or(0) > 1;
            format!(
                " <span class=\"role-tag\">{}</span>{}",
                kind.label(),
                if ambiguous {
                    format!(" <code>{}</code>", escape(&n.path))
                } else {
                    String::new()
                }
            )
        }
    };
    let relationships = graph
        .edges
        .iter()
        .filter_map(|edge| {
            let source = node_by_id.get(&edge.source)?;
            let target = node_by_id.get(&edge.target)?;
            Some(format!(
                "<li><a href=\"{}\">{}</a>{} references <a href=\"{}\">{}</a>{}</li>",
                escape(&route_href(&source.route)),
                escape(&source.label),
                relation_tag(source),
                escape(&route_href(&target.route)),
                escape(&target.label),
                relation_tag(target)
            ))
        })
        .collect::<String>();
    let files = graph
        .files
        .iter()
        .map(|f| {
            format!(
                "<li><code>{}</code> ← {}</li>",
                escape(&f.path),
                f.referenced_by
                    .iter()
                    .filter_map(|id| node_by_id.get(id))
                    .map(|n| format!(
                        "<a href=\"{}\">{}</a>",
                        escape(&route_href(&n.route)),
                        escape(&n.label)
                    ))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
        .collect::<String>();
    let files_section = if files.is_empty() {
        String::new()
    } else {
        format!("<details><summary>Repository files referenced ({})</summary><p class=\"note\">Files outside the documentation that pages link to. They are references, not knowledge relationships.</p><ul>{files}</ul></details>", graph.files.len())
    };
    let large = graph.nodes.len() > LARGE_NODES || graph.edges.len() > LARGE_EDGES;
    let note = if large {
        "<p class=\"note\">Large graph: the complete grouped index below is the primary view. Your browser’s Find searches every title and path.</p>"
    } else {
        ""
    };
    let visual_open = if large { "" } else { " open" };
    let present = |kind: ArtifactKind| graph.nodes.iter().any(|n| n.artifact == kind);
    let mut kind_filters = String::new();
    for (kind, label) in [
        (ArtifactKind::Documentation, "Documentation"),
        (ArtifactKind::AgentInstructions, "Agent instructions"),
        (ArtifactKind::Skill, "Skills"),
    ] {
        if present(kind) {
            kind_filters.push_str(&format!(
                "<label><input type=\"checkbox\" data-filter-kind=\"{}\" checked> {label}</label>",
                kind.as_str()
            ));
        }
    }
    if !graph.files.is_empty() {
        kind_filters.push_str("<label><input type=\"checkbox\" data-filter-kind=\"repository_file\"> Repository references</label>");
    }
    if !exposures.is_empty() {
        kind_filters.push_str(&format!(
            "<label title=\"Manifests that only expose a canonical skill to another tool\"><input type=\"checkbox\" data-filter-exposures> Skill exposures ({})</label>",
            exposures.len()
        ));
    }
    if !layout.unlinked.is_empty() {
        // Unlinked nodes are hidden by default only when they would crowd out the connected graph.
        let crowding = layout.unlinked.len() > 12
            && layout.unlinked.len() > graph.nodes.len() - exposures.len() - layout.unlinked.len();
        kind_filters.push_str(&format!(
            "<label title=\"Nodes with no relationship to anything else; all remain in the index below\"><input type=\"checkbox\" data-filter-unlinked{}> Unlinked ({})</label>",
            if crowding { "" } else { " checked" },
            layout.unlinked.len()
        ));
    }
    let mut role_filters = String::new();
    for role in RECOMMENDED_ROLES
        .iter()
        .chain(std::iter::once(&KnowledgeRole::Other))
    {
        if graph
            .nodes
            .iter()
            .any(|n| n.artifact == ArtifactKind::Documentation && n.role == *role)
        {
            role_filters.push_str(&format!(
                "<label><input type=\"checkbox\" data-filter-role=\"{}\" checked> {}</label>",
                role.as_str(),
                role.label()
            ));
        }
    }
    let shown = graph.nodes.len() - exposures.len();
    let hidden_note = if exposures.is_empty() {
        String::new()
    } else {
        format!(
            " · {} skill exposure{} collapsed into canonical skills",
            exposures.len(),
            if exposures.len() == 1 { "" } else { "s" }
        )
    };
    let summary = if kinds == 0 {
        format!(
            "{} documents · {} relationships",
            graph.nodes.len(),
            graph.edges.len()
        )
    } else {
        format!(
            "{shown} nodes ({} documentation, {} agent-facing) · {} relationships{hidden_note}",
            graph.nodes.len() - kinds,
            kinds - exposures.len(),
            graph.edges.len()
        )
    };
    let agents_link = if kinds > 0 {
        "<a href=\"../agents/\">Agents</a>"
    } else {
        ""
    };
    let content = format!(
        "<main id=\"main\" tabindex=\"-1\" class=\"graph-page\"><header class=\"graph-header\"><nav class=\"graph-nav\" aria-label=\"Entwine views\"><a href=\"../../\">Documentation</a><a href=\"../knowledge/\">Knowledge</a>{agents_link}</nav><h1>Project graph</h1><p>{summary}</p></header>{note}<details{visual_open} class=\"graph-details\"><summary>Visual graph</summary><div class=\"graph-app\" id=\"graph-app\"><p class=\"graph-hint\">Select a node to open it. Arrows point to referenced pages. With scripting on you can pan, zoom, drag, filter, and focus a node’s neighbors.</p>{legend}<div class=\"graph-controls\" hidden><fieldset><legend>Show</legend>{kind_filters}</fieldset><fieldset><legend>Document roles</legend>{role_filters}</fieldset><div class=\"graph-actions\" role=\"group\" aria-label=\"Graph view\"><button type=\"button\" data-action=\"zoom-in\" aria-label=\"Zoom in\">＋</button><button type=\"button\" data-action=\"zoom-out\" aria-label=\"Zoom out\">－</button><button type=\"button\" data-action=\"fit\">Fit</button><button type=\"button\" data-action=\"reset\">Reset layout</button><button type=\"button\" data-action=\"focus\" aria-pressed=\"false\" disabled>Focus neighbors</button></div></div><div class=\"graph-workspace\"><div class=\"graph-scroll\" tabindex=\"0\" aria-label=\"Document graph. Scroll to explore; with scripting: drag to pan, wheel to zoom, arrow keys to pan, plus and minus to zoom, Escape to clear selection.\">{svg}</div><aside class=\"graph-inspector\" aria-label=\"Selected node\" aria-live=\"polite\" hidden><p class=\"graph-empty\">Select a node to see its details and neighbors.</p></aside></div><script type=\"application/json\" id=\"graph-data\">{data}</script></div></details><div class=\"graph-index\"><details><summary>Documents and artifacts ({})</summary>{groups}</details><details open><summary>Relationships ({})</summary><ul>{relationships}</ul></details>{files_section}</div></main>",
        graph.nodes.len(),
        graph.edges.len(),
        legend = legend(graph)
    );
    StaticFile {
        path: "__entwine/graph/index.html".into(),
        contents: frame_with(
            "Project graph",
            ROUTE,
            &content,
            &format!(
                "<script defer src=\"{}\"></script>",
                escape(&relative_url(ROUTE, "/__entwine/graph.js"))
            ),
        )
        .into_bytes(),
    }
}
