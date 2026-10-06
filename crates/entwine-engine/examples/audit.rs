//! Read-only audit/benchmark helper. Never publishes invalid compilations.
use std::{path::Path, time::Instant};
fn main() {
    let project = std::env::args().nth(1).expect("usage: audit <project>");
    let started = Instant::now();
    let c = entwine_engine::compile(Path::new(&project)).unwrap();
    let compile_ms = started.elapsed().as_secs_f64() * 1000.0;
    let started = Instant::now();
    let graph = entwine_engine::render_graph(&c.graph);
    let graph_ms = started.elapsed().as_secs_f64() * 1000.0;
    let started = Instant::now();
    let files = entwine_engine::render(&c);
    let render_ms = started.elapsed().as_secs_f64() * 1000.0;
    println!("{}", serde_json::to_string_pretty(&serde_json::json!({
        "documents": c.knowledge.documents.len(),
        "relationships": c.knowledge.relations.len(),
        "repository_references": c.knowledge.documents.iter().map(|d| d.repository_references.len()).sum::<usize>(),
        "references": c.knowledge.documents.iter().flat_map(|d| &d.repository_references).collect::<Vec<_>>(),
        "coverage": entwine_core::coverage(&c.knowledge.documents),
        "diagnostics": c.diagnostics,
        "pages": c.site.pages.len() + 2,
        "compile_ms": compile_ms, "graph_ms": graph_ms, "render_ms": render_ms,
        "output_bytes": files.iter().map(|f| f.contents.len()).sum::<usize>(),
        "graph_bytes": graph.contents.len(),
        "context_bytes": entwine_engine::context_json(&c.context).unwrap().len(),
        "schema": c.context.schema_version,
    })).unwrap());
}
