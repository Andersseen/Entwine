//! Deterministic force-directed layout with kind/role clusters.
//!
//! Only IEEE arithmetic and `sqrt` are used (no trigonometry), so the same input yields the
//! same coordinates on every platform. The browser may later refine positions, but this is
//! the stable initial layout and the complete static fallback.
use entwine_core::{ArtifactKind, DocumentId, GraphModel, GraphNode};
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
    /// Keys of nodes with no visible relationship. They are packed into a tidy block instead of
    /// being scattered, and the browser leaves them where they are.
    pub unlinked: std::collections::BTreeSet<String>,
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
    // Exposures of a canonical skill are collapsed into it: they take no part in the layout
    // and sit beside their canonical node.
    let exposure_of: BTreeMap<&str, &str> = graph
        .nodes
        .iter()
        .filter_map(|n| Some((n.id.0.as_str(), n.exposure_of.as_ref()?.0.as_str())))
        .collect();
    let mut all: Vec<Item> = graph
        .nodes
        .iter()
        .filter(|n| !exposure_of.contains_key(n.id.0.as_str()))
        .map(|n| Item {
            key: n.id.0.clone(),
            cluster: cluster(n),
        })
        .collect();
    if include_files {
        all.extend(graph.files.iter().map(|f| Item {
            key: file_key(&f.path),
            cluster: FILE_CLUSTER,
        }));
    }
    all.sort_by(|a, b| (a.cluster, &a.key).cmp(&(b.cluster, &b.key)));
    let known: BTreeMap<&str, usize> = all
        .iter()
        .enumerate()
        .map(|(i, item)| (item.key.as_str(), i))
        .collect();
    let mut pairs: Vec<(&str, &str)> = graph
        .edges
        .iter()
        .filter(|e| e.source != e.target)
        .map(|e| (e.source.0.as_str(), e.target.0.as_str()))
        .filter(|(a, b)| known.contains_key(a) && known.contains_key(b))
        .collect();
    let file_keys: Vec<(String, &DocumentId)> = if include_files {
        graph
            .files
            .iter()
            .flat_map(|f| f.referenced_by.iter().map(|d| (file_key(&f.path), d)))
            .collect()
    } else {
        Vec::new()
    };
    pairs.extend(
        file_keys
            .iter()
            .filter(|(_, d)| known.contains_key(d.0.as_str()))
            .map(|(k, d)| (d.0.as_str(), k.as_str())),
    );
    let mut degrees: BTreeMap<String, usize> =
        all.iter().map(|item| (item.key.clone(), 0usize)).collect();
    for (a, b) in &pairs {
        *degrees.get_mut(*a).unwrap() += 1;
        *degrees.get_mut(*b).unwrap() += 1;
    }
    // Only connected items run through the force simulation.
    let (items, isolates): (Vec<&Item>, Vec<&Item>) =
        all.iter().partition(|item| degrees[&item.key] > 0);
    let index: BTreeMap<&str, usize> = items
        .iter()
        .enumerate()
        .map(|(i, item)| (item.key.as_str(), i))
        .collect();
    let edges: Vec<(usize, usize)> = pairs
        .iter()
        .filter_map(|(a, b)| Some((*index.get(a)?, *index.get(b)?)))
        .collect();
    let n = items.len();
    // Each connected component is laid out on its own and the components are then packed, so
    // unrelated groups never repel each other across the whole canvas.
    let mut parent: Vec<usize> = (0..n).collect();
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    for (a, b) in &edges {
        let (ra, rb) = (root(&mut parent, *a), root(&mut parent, *b));
        if ra != rb {
            parent[ra.max(rb)] = ra.min(rb);
        }
    }
    let mut members: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..n {
        let r = root(&mut parent, i);
        members.entry(r).or_default().push(i);
    }
    let mut components: Vec<Vec<usize>> = members.into_values().collect();
    components.sort_by(|a, b| b.len().cmp(&a.len()).then(a[0].cmp(&b[0])));
    let mut boxes: Vec<(Vec<usize>, Vec<Point>, f64, f64)> = Vec::new();
    for component in components {
        let local: BTreeMap<usize, usize> =
            component.iter().enumerate().map(|(l, g)| (*g, l)).collect();
        let clusters: Vec<usize> = component.iter().map(|g| items[*g].cluster).collect();
        let inner: Vec<(usize, usize)> = edges
            .iter()
            .filter_map(|(a, b)| Some((*local.get(a)?, *local.get(b)?)))
            .collect();
        let mut points = simulate(&clusters, &inner);
        let min_x = points.iter().map(|p| p.x).fold(f64::MAX, f64::min);
        let min_y = points.iter().map(|p| p.y).fold(f64::MAX, f64::min);
        for p in &mut points {
            p.x -= min_x;
            p.y -= min_y;
        }
        let width = points.iter().map(|p| p.x).fold(0.0, f64::max);
        let height = points.iter().map(|p| p.y).fold(0.0, f64::max);
        boxes.push((component, points, width, height));
    }
    // Shelf packing: the largest component first, rows no wider than needed.
    let area: f64 = boxes
        .iter()
        .map(|(_, _, w, h)| (w + COMPONENT_GAP) * (h + COMPONENT_GAP))
        .sum();
    let widest = boxes.iter().map(|(_, _, w, _)| *w).fold(0.0, f64::max);
    let row_width = widest.max(area.sqrt() * 1.25);
    let mut position = vec![Point { x: 0.0, y: 0.0 }; n];
    let (mut cursor_x, mut cursor_y, mut row_height) = (0.0, 0.0, 0.0f64);
    for (component, points, width, height) in &boxes {
        if cursor_x > 0.0 && cursor_x + width > row_width {
            cursor_x = 0.0;
            cursor_y += row_height + COMPONENT_GAP;
            row_height = 0.0;
        }
        for (g, p) in component.iter().zip(points) {
            position[*g] = Point {
                x: cursor_x + p.x,
                y: cursor_y + p.y,
            };
        }
        cursor_x += width + COMPONENT_GAP;
        row_height = row_height.max(*height);
    }
    let mut placed: BTreeMap<String, Point> = items
        .iter()
        .zip(&position)
        .map(|(item, p)| (item.key.clone(), *p))
        .collect();
    // Unlinked nodes: one compact, labelled-by-cluster block per cluster below the graph.
    let (mut max_y, min_x) = if n == 0 {
        (-ISOLATE_PITCH_Y, 0.0)
    } else {
        (
            position.iter().map(|p| p.y).fold(f64::MIN, f64::max),
            position.iter().map(|p| p.x).fold(f64::MAX, f64::min),
        )
    };
    let mut cursor_x = min_x;
    let mut row_top = max_y + ISOLATE_PITCH_Y * 1.6;
    let mut row_bottom = row_top;
    let total_width = if n == 0 {
        0.0
    } else {
        position.iter().map(|p| p.x).fold(f64::MIN, f64::max) - min_x
    }
    .max(ISOLATE_PITCH_X * 6.0);
    for cluster_id in 0..10 {
        let group: Vec<&&Item> = isolates
            .iter()
            .filter(|i| i.cluster == cluster_id)
            .collect();
        if group.is_empty() {
            continue;
        }
        let columns = ((group.len() as f64 * 1.5).sqrt().ceil() as usize).clamp(1, 10);
        let rows = group.len().div_ceil(columns);
        let width = columns as f64 * ISOLATE_PITCH_X;
        if cursor_x > min_x && cursor_x + width > min_x + total_width {
            cursor_x = min_x;
            row_top = row_bottom + ISOLATE_PITCH_Y * 0.6;
        }
        for (i, item) in group.iter().enumerate() {
            placed.insert(
                item.key.clone(),
                Point {
                    x: cursor_x + (i % columns) as f64 * ISOLATE_PITCH_X,
                    y: row_top + (i / columns) as f64 * ISOLATE_PITCH_Y,
                },
            );
        }
        row_bottom = row_bottom.max(row_top + rows as f64 * ISOLATE_PITCH_Y);
        cursor_x += width + ISOLATE_PITCH_X * 0.5;
    }
    max_y = max_y.max(row_bottom);
    let _ = max_y;
    // Exposures sit just beside their canonical node.
    let mut beside: BTreeMap<&str, usize> = BTreeMap::new();
    for node in &graph.nodes {
        let Some(canonical) = exposure_of.get(node.id.0.as_str()) else {
            continue;
        };
        let Some(base) = placed.get(*canonical).copied() else {
            continue;
        };
        let slot = beside.entry(canonical).or_default();
        let (dx, dy) = EXPOSURE_OFFSETS[*slot % EXPOSURE_OFFSETS.len()];
        *slot += 1;
        placed.insert(
            node.id.0.clone(),
            Point {
                x: base.x + dx,
                y: base.y + dy,
            },
        );
        degrees.insert(node.id.0.clone(), 1);
    }
    let unlinked = isolates.iter().map(|i| i.key.clone()).collect();
    if placed.is_empty() {
        return Layout {
            width: 600.0,
            height: 400.0,
            positions: BTreeMap::new(),
            degrees,
            unlinked,
        };
    }
    let margin = 110.0;
    let min_x = placed.values().map(|p| p.x).fold(f64::MAX, f64::min);
    let min_y = placed.values().map(|p| p.y).fold(f64::MAX, f64::min);
    let max_x = placed.values().map(|p| p.x).fold(f64::MIN, f64::max);
    let max_y = placed.values().map(|p| p.y).fold(f64::MIN, f64::max);
    let positions = placed
        .into_iter()
        .map(|(key, p)| {
            (
                key,
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
        unlinked,
    }
}

/// Fruchterman–Reingold for one connected component: each cluster spirals around its slot on a
/// ring, with a cooling schedule, a weak pull to the cluster center, and light gravity.
fn simulate(clusters: &[usize], edges: &[(usize, usize)]) -> Vec<Point> {
    let n = clusters.len();
    let ring = 160.0 + 40.0 * (n as f64).sqrt();
    let mut counts = [0usize; 10];
    let mut spiral: [(f64, f64); 10] = [(1.0, 0.0); 10];
    let centers: Vec<Point> = SLOTS
        .iter()
        .map(|(x, y)| Point {
            x: x * ring,
            y: y * ring,
        })
        .collect();
    let mut position: Vec<Point> = Vec::with_capacity(n);
    for &c in clusters {
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
        for (a, b) in edges {
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
        for (i, &c) in clusters.iter().enumerate() {
            let center = centers[c];
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
    position
}
const COMPONENT_GAP: f64 = 110.0;

const ISOLATE_PITCH_X: f64 = 150.0;
const ISOLATE_PITCH_Y: f64 = 74.0;
/// Offsets for exposures stacked beside a canonical node.
const EXPOSURE_OFFSETS: [(f64, f64); 6] = [
    (36.0, 30.0),
    (-36.0, 30.0),
    (44.0, -26.0),
    (-44.0, -26.0),
    (0.0, 50.0),
    (0.0, -48.0),
];

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
            location: None,
            exposure_of: None,
            exposures: 0,
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
