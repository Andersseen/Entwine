//! A fixed radial layout: the most connected document anchors concentric rings.
//! No randomness, animation, physics, or browser layout is involved.
use entwine_core::{DocumentId, GraphModel};
use std::{collections::BTreeMap, f64::consts::TAU};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Point {
    pub x: f64,
    pub y: f64,
}

pub(crate) struct Layout {
    pub size: f64,
    pub positions: BTreeMap<DocumentId, Point>,
    pub hub: Option<DocumentId>,
}

pub(crate) fn layout(graph: &GraphModel) -> Layout {
    if graph.nodes.len() > 12 {
        return role_layout(graph);
    }
    let mut degrees: BTreeMap<_, usize> = graph
        .nodes
        .iter()
        .map(|node| (node.id.clone(), 0))
        .collect();
    for edge in &graph.edges {
        if let Some(degree) = degrees.get_mut(&edge.source) {
            *degree += 1;
        }
        if let Some(degree) = degrees.get_mut(&edge.target) {
            *degree += 1;
        }
    }
    let mut ordered: Vec<_> = degrees.into_iter().collect();
    ordered.sort_by(|(a, da), (b, db)| db.cmp(da).then_with(|| a.cmp(b)));
    let hub = ordered.first().map(|(id, _)| id.clone());
    let ranks: BTreeMap<_, _> = graph
        .nodes
        .iter()
        .map(|n| (n.id.clone(), n.role.rank()))
        .collect();
    let mut remaining: Vec<_> = ordered.into_iter().skip(1).map(|(id, _)| id).collect();
    // Role-aware but deterministic: convention roles first, then by id.
    remaining.sort_by(|a, b| (ranks.get(a), a).cmp(&(ranks.get(b), b)));
    let mut rings = Vec::new();
    let mut start = 0;
    let mut ring = 1;
    while start < remaining.len() {
        let end = (start + ring * 8).min(remaining.len());
        rings.push((165.0 * ring as f64, &remaining[start..end]));
        start = end;
        ring += 1;
    }
    let radius = rings.last().map_or(35.0, |(radius, _)| *radius);
    let size = (radius + 105.0) * 2.0;
    let center = size / 2.0;
    let mut positions = BTreeMap::new();
    if let Some(id) = &hub {
        positions.insert(
            id.clone(),
            Point {
                x: center,
                y: center,
            },
        );
    }
    for (ring_index, (radius, nodes)) in rings.iter().enumerate() {
        for (index, id) in nodes.iter().enumerate() {
            // Alternate ring rotation so radial edges do not all share spokes.
            let angle = TAU * (index as f64 + if ring_index % 2 == 0 { 0.0 } else { 0.5 })
                / nodes.len() as f64
                - TAU / 4.0;
            positions.insert(
                (*id).clone(),
                Point {
                    x: (center + radius * angle.cos()).round(),
                    y: (center + radius * angle.sin()).round(),
                },
            );
        }
    }
    Layout {
        size,
        positions,
        hub,
    }
}

/// Role bands group presentation only; edges still come from Markdown.
fn role_layout(graph: &GraphModel) -> Layout {
    let mut ordered: Vec<_> = graph.nodes.iter().collect();
    ordered.sort_by(|a, b| (a.role.rank(), &a.id).cmp(&(b.role.rank(), &b.id)));
    let columns = 5usize;
    let mut rows = Vec::new();
    for rank in 0..=6 {
        let group: Vec<_> = ordered.iter().filter(|n| n.role.rank() == rank).collect();
        for chunk in group.chunks(columns) {
            rows.push(chunk.to_vec());
        }
    }
    let size = ((rows.len() as f64 * 160.0 + 140.0).max(1100.0)).ceil();
    let mut positions = BTreeMap::new();
    for (row, nodes) in rows.iter().enumerate() {
        for (column, node) in nodes.iter().enumerate() {
            positions.insert(
                node.id.clone(),
                Point {
                    x: size / 2.0 + (column as f64 - (nodes.len() as f64 - 1.0) / 2.0) * 190.0,
                    y: 100.0 + row as f64 * 160.0,
                },
            );
        }
    }
    Layout {
        size,
        positions,
        hub: None,
    }
}

/// Clip edges at the circumference and separate opposite directions with curves.
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
        target.x - end_dx / end_distance * (target_radius + 3.0),
        target.y - end_dy / end_distance * (target_radius + 3.0)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use entwine_core::*;

    #[test]
    fn the_most_connected_document_is_centered() {
        let nodes = ["a", "b", "c"]
            .into_iter()
            .map(|name| GraphNode {
                id: DocumentId(name.into()),
                label: name.into(),
                route: Route::home(),
                metadata: DocumentMetadata::default(),
                role: KnowledgeRole::Other,
            })
            .collect();
        let graph = GraphModel {
            nodes,
            edges: [("a", "b"), ("b", "c")]
                .into_iter()
                .map(|(source, target)| Relation {
                    source: DocumentId(source.into()),
                    target: DocumentId(target.into()),
                    kind: RelationKind::References,
                })
                .collect(),
        };
        let result = layout(&graph);
        assert_eq!(result.hub, Some(DocumentId("b".into())));
        let center = result.positions[&DocumentId("b".into())];
        assert_eq!(center.x, result.size / 2.0);
        assert_eq!(center.y, result.size / 2.0);
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
    fn layouts_fit_the_canvas_and_have_distinct_two_dimensional_positions() {
        for count in [1, 5, 20, 100] {
            let graph = GraphModel {
                nodes: (0..count)
                    .map(|i| GraphNode {
                        id: DocumentId(format!("{i:03}.md")),
                        label: format!("Document {i}"),
                        route: Route::home(),
                        metadata: DocumentMetadata::default(),
                        role: KnowledgeRole::Other,
                    })
                    .collect(),
                edges: Vec::new(),
            };
            let result = layout(&graph);
            assert_eq!(result.positions.len(), count);
            let unique: std::collections::BTreeSet<_> = result
                .positions
                .values()
                .map(|p| (p.x as i32, p.y as i32))
                .collect();
            assert_eq!(unique.len(), count);
            for point in result.positions.values() {
                assert!(
                    point.x >= 90.0
                        && point.x <= result.size - 90.0
                        && point.y >= 90.0
                        && point.y <= result.size - 90.0
                );
            }
            if count > 1 {
                assert!(
                    unique
                        .iter()
                        .map(|(x, _)| x)
                        .collect::<std::collections::BTreeSet<_>>()
                        .len()
                        > 1
                );
            }
            let mut reordered = graph.clone();
            reordered.nodes.reverse();
            assert_eq!(
                unique,
                layout(&reordered)
                    .positions
                    .values()
                    .map(|p| (p.x as i32, p.y as i32))
                    .collect()
            );
        }
    }
}
