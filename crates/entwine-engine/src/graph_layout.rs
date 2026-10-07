//! Deterministic force-directed layout with kind/role clusters.
//!
//! Only IEEE arithmetic and `sqrt` are used (no trigonometry), so the same input yields the
//! same coordinates on every platform. The browser may later refine positions, but this is
//! the stable initial layout and the complete static fallback.
use entwine_core::{ArtifactKind, GraphModel, GraphNode};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Point {
    pub x: f64,
    pub y: f64,
}

pub(crate) struct Layout {
    pub width: f64,
    pub height: f64,
    /// Keyed by document id, or `file:<path>` for repository file nodes.
    pub positions: BTreeMap<String, Point>,
    pub degrees: BTreeMap<String, usize>,
}

pub(crate) fn file_key(path: &str) -> String {
    format!("file:{path}")
}

/// Ten fixed cluster slots at 36° steps (unit vectors), so empty clusters never shift others.
const SLOTS: [(f64, f64); 10] = [
    (0.0, -1.0),
    (0.587_785_252_292_473, -0.809_016_994_374_947_4),
    (0.951_056_516_295_153_5, -0.309_016_994_374_947_5),
    (0.951_056_516_295_153_5, 0.309_016_994_374_947_5),
    (0.587_785_252_292_473, 0.809_016_994_374_947_4),
    (0.0, 1.0),
    (-0.587_785_252_292_473, 0.809_016_994_374_947_4),
    (-0.951_056_516_295_153_5, 0.309_016_994_374_947_5),
    (-0.951_056_516_295_153_5, -0.309_016_994_374_947_5),
    (-0.587_785_252_292_473, -0.809_016_994_374_947_4),
];
// cos/sin of the golden angle, for a phyllotaxis spiral built by repeated rotation.
const GOLDEN_COS: f64 = -0.737_368_878_078_320_1;
const GOLDEN_SIN: f64 = 0.675_490_294_261_524_2;

/// Cluster slot: documentation by role rank (0–6), then instructions, skills, repository files.
pub(crate) fn cluster(node: &GraphNode) -> usize {
    match node.artifact {
        ArtifactKind::Documentation => node.role.rank() as usize,
        ArtifactKind::AgentInstructions => 7,
        ArtifactKind::Skill => 8,
    }
}
const FILE_CLUSTER: usize = 9;

pub(crate) fn layout(graph: &GraphModel, include_files: bool) -> Layout {
    struct Item {
        key: String,
        cluster: usize,
    }
    let mut items: Vec<Item> = graph
        .nodes
        .iter()
        .map(|n| Item {
            key: n.id.0.clone(),
            cluster: cluster(n),
        })
        .collect();
    if include_files {
        items.extend(graph.files.iter().map(|f| Item {
            key: file_key(&f.path),
            cluster: FILE_CLUSTER,
        }));
    }
    items.sort_by(|a, b| (a.cluster, &a.key).cmp(&(b.cluster, &b.key)));
    let index: BTreeMap<&str, usize> = items
        .iter()
        .enumerate()
        .map(|(i, item)| (item.key.as_str(), i))
        .collect();
    let mut edges: Vec<(usize, usize)> = graph
        .edges
        .iter()
        .filter_map(|e| {
            Some((
                *index.get(e.source.0.as_str())?,
                *index.get(e.target.0.as_str())?,
            ))
        })
        .filter(|(a, b)| a != b)
        .collect();
    if include_files {
        for file in &graph.files {
            let Some(target) = index.get(file_key(&file.path).as_str()) else {
                continue;
            };
            for source in &file.referenced_by {
                if let Some(source) = index.get(source.0.as_str()) {
                    edges.push((*source, *target));
                }
            }
        }
    }
    let mut degrees = BTreeMap::new();
    for item in &items {
        degrees.insert(item.key.clone(), 0usize);
    }
    for (a, b) in &edges {
        *degrees.get_mut(&items[*a].key).unwrap() += 1;
        *degrees.get_mut(&items[*b].key).unwrap() += 1;
    }
    let n = items.len();
    if n == 0 {
        return Layout {
            width: 600.0,
            height: 400.0,
            positions: BTreeMap::new(),
            degrees,
        };
    }
    // Initial placement: each cluster is a spiral around its slot on a ring.
    let ring = 160.0 + 40.0 * (n as f64).sqrt();
    let mut counts = [0usize; 10];
    let mut position: Vec<Point> = Vec::with_capacity(n);
    let mut spiral: [(f64, f64); 10] = [(1.0, 0.0); 10];
    let centers: Vec<Point> = SLOTS
        .iter()
        .map(|(x, y)| Point {
            x: x * ring,
            y: y * ring,
        })
        .collect();
    for item in &items {
        let c = item.cluster;
        let i = counts[c];
        counts[c] += 1;
        let (dx, dy) = spiral[c];
        let radius = 62.0 * (i as f64 + 0.5).sqrt();
        position.push(Point {
            x: centers[c].x + dx * radius,
            y: centers[c].y + dy * radius,
        });
        spiral[c] = (
            dx * GOLDEN_COS - dy * GOLDEN_SIN,
            dx * GOLDEN_SIN + dy * GOLDEN_COS,
        );
    }
    // Fruchterman–Reingold with a cooling schedule, a weak pull to the cluster center,
    // and gravity to the origin so disconnected components stay near each other.
    let k = 165.0;
    let iterations = if n > 300 { 120 } else { 260 };
    for step in 0..iterations {
        let temperature = (ring * 0.18) * (1.0 - step as f64 / iterations as f64) + 1.0;
        let mut force = vec![Point { x: 0.0, y: 0.0 }; n];
        for a in 0..n {
            for b in (a + 1)..n {
                let mut dx = position[a].x - position[b].x;
                let mut dy = position[a].y - position[b].y;
                let mut d2 = dx * dx + dy * dy;
                if d2 < 0.01 {
                    // Coincident nodes separate along a fixed, index-derived axis.
                    dx = 0.1 * (1.0 + (a % 3) as f64);
                    dy = 0.1 * (1.0 + (b % 3) as f64);
                    d2 = dx * dx + dy * dy;
                }
                let d = d2.sqrt();
                let repulse = k * k / d;
                let (fx, fy) = (dx / d * repulse, dy / d * repulse);
                force[a].x += fx;
                force[a].y += fy;
                force[b].x -= fx;
                force[b].y -= fy;
            }
        }
        for (a, b) in &edges {
            let dx = position[*a].x - position[*b].x;
            let dy = position[*a].y - position[*b].y;
            let d = (dx * dx + dy * dy).sqrt().max(0.01);
            let attract = d * d / k * 0.85;
            let (fx, fy) = (dx / d * attract, dy / d * attract);
            force[*a].x -= fx;
            force[*a].y -= fy;
            force[*b].x += fx;
            force[*b].y += fy;
        }
        for (i, item) in items.iter().enumerate() {
            let center = centers[item.cluster];
            force[i].x += (center.x - position[i].x) * 0.12;
            force[i].y += (center.y - position[i].y) * 0.12;
            force[i].x -= position[i].x * 0.05;
            force[i].y -= position[i].y * 0.05;
            let magnitude = (force[i].x * force[i].x + force[i].y * force[i].y).sqrt();
            if magnitude > 0.0 {
                let limited = magnitude.min(temperature);
                position[i].x += force[i].x / magnitude * limited;
                position[i].y += force[i].y / magnitude * limited;
            }
        }
    }
    let margin = 110.0;
    let min_x = position.iter().map(|p| p.x).fold(f64::MAX, f64::min);
    let min_y = position.iter().map(|p| p.y).fold(f64::MAX, f64::min);
    let max_x = position.iter().map(|p| p.x).fold(f64::MIN, f64::max);
    let max_y = position.iter().map(|p| p.y).fold(f64::MIN, f64::max);
    let positions = items
        .iter()
        .zip(&position)
        .map(|(item, p)| {
            (
                item.key.clone(),
                Point {
                    x: (p.x - min_x + margin).round(),
                    y: (p.y - min_y + margin).round(),
                },
            )
        })
        .collect();
    Layout {
        width: (max_x - min_x + margin * 2.0).round().max(520.0),
        height: (max_y - min_y + margin * 2.0).round().max(360.0),
        positions,
        degrees,
    }
}

/// Node radius from its connectivity: hubs read as hubs.
pub(crate) fn radius(degree: usize) -> f64 {
    13.0 + 2.0 * degree.min(6) as f64
}

/// Clip edges at the node outlines and bend them slightly so opposite directions separate.
/// The browser enhancement uses the identical construction.
pub(crate) fn edge_path(
    source: Point,
    target: Point,
    source_radius: f64,
    target_radius: f64,
) -> String {
    let dx = target.x - source.x;
    let dy = target.y - source.y;
    let distance = dx.hypot(dy);
    if distance < 1.0 {
        return format!(
            "M {:.1} {:.1} C {:.1} {:.1}, {:.1} {:.1}, {:.1} {:.1}",
            source.x - 14.0,
            source.y - source_radius,
            source.x - 65.0,
            source.y - 90.0,
            source.x + 65.0,
            source.y - 90.0,
            source.x + 14.0,
            source.y - target_radius
        );
    }
    let bend = distance * 0.12;
    let control = Point {
        x: (source.x + target.x) / 2.0 - dy / distance * bend,
        y: (source.y + target.y) / 2.0 + dx / distance * bend,
    };
    let start_dx = control.x - source.x;
    let start_dy = control.y - source.y;
    let start_distance = start_dx.hypot(start_dy);
    let end_dx = target.x - control.x;
    let end_dy = target.y - control.y;
    let end_distance = end_dx.hypot(end_dy);
    format!(
        "M {:.1} {:.1} Q {:.1} {:.1} {:.1} {:.1}",
        source.x + start_dx / start_distance * source_radius,
        source.y + start_dy / start_distance * source_radius,
        control.x,
        control.y,
        target.x - end_dx / end_distance * (target_radius + 4.0),
        target.y - end_dy / end_distance * (target_radius + 4.0)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use entwine_core::*;

    fn node(id: &str, role: KnowledgeRole, artifact: ArtifactKind) -> GraphNode {
        GraphNode {
            id: DocumentId(id.into()),
            label: id.into(),
            route: Route::home(),
            metadata: DocumentMetadata::default(),
            role,
            artifact,
            path: id.into(),
            scope: None,
        }
    }
    fn graph(count: usize) -> GraphModel {
        GraphModel {
            nodes: (0..count)
                .map(|i| {
                    node(
                        &format!("{i:03}.md"),
                        if i % 3 == 0 {
                            KnowledgeRole::Spec
                        } else {
                            KnowledgeRole::Other
                        },
                        if i % 7 == 6 {
                            ArtifactKind::Skill
                        } else {
                            ArtifactKind::Documentation
                        },
                    )
                })
                .collect(),
            edges: (1..count)
                .map(|i| Relation {
                    source: DocumentId(format!("{:03}.md", i / 2)),
                    target: DocumentId(format!("{i:03}.md")),
                    kind: RelationKind::References,
                })
                .collect(),
            files: Vec::new(),
        }
    }

    #[test]
    fn directed_edges_and_self_references_have_finite_distinct_paths() {
        let a = Point { x: 100.0, y: 100.0 };
        let b = Point { x: 300.0, y: 200.0 };
        let paths = [
            edge_path(a, b, 19.0, 25.0),
            edge_path(b, a, 25.0, 19.0),
            edge_path(a, a, 19.0, 19.0),
        ];
        assert_ne!(paths[0], paths[1]);
        for path in &paths {
            assert!(!path.contains("NaN") && !path.contains("inf"));
        }
        assert!(paths[2].contains(" C "));
    }

    #[test]
    fn layouts_are_deterministic_finite_distinct_and_inside_the_canvas() {
        for count in [1, 5, 20, 100, 250] {
            let g = graph(count);
            let first = layout(&g, false);
            assert_eq!(first.positions.len(), count);
            let again = layout(&g, false);
            let key = |l: &Layout| {
                l.positions
                    .iter()
                    .map(|(k, p)| (k.clone(), p.x as i64, p.y as i64))
                    .collect::<Vec<_>>()
            };
            assert_eq!(key(&first), key(&again));
            let mut reordered = g.clone();
            reordered.nodes.reverse();
            assert_eq!(key(&first), key(&layout(&reordered, false)));
            let distinct: std::collections::BTreeSet<_> = first
                .positions
                .values()
                .map(|p| (p.x as i64, p.y as i64))
                .collect();
            assert_eq!(distinct.len(), count, "{count} nodes overlap exactly");
            for p in first.positions.values() {
                assert!(p.x.is_finite() && p.y.is_finite());
                assert!(p.x >= 100.0 && p.x <= first.width - 100.0);
                assert!(p.y >= 100.0 && p.y <= first.height - 100.0);
            }
        }
    }

    #[test]
    fn nodes_keep_a_readable_minimum_distance() {
        let result = layout(&graph(60), false);
        let points: Vec<_> = result.positions.values().collect();
        let mut closest = f64::MAX;
        for (i, a) in points.iter().enumerate() {
            for b in &points[i + 1..] {
                closest = closest.min((a.x - b.x).hypot(a.y - b.y));
            }
        }
        assert!(closest >= 30.0, "closest pair is {closest:.1}px apart");
    }
}
