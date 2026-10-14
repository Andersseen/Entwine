//! Compare page rendering only, using one already projected kitchen-sink SiteModel.
//! Run with `cargo run --release -p entwine-engine --example renderer_benchmark`.

use entwine_core::{PageReference, RendererKind, Route};
use entwine_engine::{compile, render_site_with_renderer};
use std::{path::PathBuf, time::Instant};

fn main() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/kitchen-sink");
    let mut site = compile(&project).expect("kitchen-sink compiles").site;
    let prototype = site.pages[0].clone();

    println!("Pages | Built-in ms | Flowview ms | Built-in bytes | Flowview bytes");
    for count in [10, 100, 500, 1000] {
        site.pages = (0..count)
            .map(|index| {
                let mut page = prototype.clone();
                page.route = Route::from_source(&format!("bench/page-{index}.md")).unwrap();
                page.title = format!("Benchmark page {index}");
                page.html = format!(
                    "<h1 id=\"page-{index}\">Benchmark page {index}</h1><p>{}</p>",
                    "Content ".repeat(24)
                );
                page.headings.clear();
                page.references = vec![PageReference {
                    title: "Reference target".into(),
                    route: Route::from_source("reference.md").unwrap(),
                }];
                page.backlinks = vec![PageReference {
                    title: "Backlink source".into(),
                    route: Route::from_source("backlink.md").unwrap(),
                }];
                page.source_path = Some(format!("docs/bench/page-{index}.md"));
                page.source_url = None;
                page.graph_id = None;
                page
            })
            .collect();

        let builtin = measure(&site, RendererKind::Builtin);
        let flowview = measure(&site, RendererKind::Flowview);
        println!(
            "{count} | {:.2} | {:.2} | {} | {}",
            builtin.0.as_secs_f64() * 1000.0,
            flowview.0.as_secs_f64() * 1000.0,
            builtin.1,
            flowview.1
        );
    }
}

fn measure(site: &entwine_core::SiteModel, renderer: RendererKind) -> (std::time::Duration, usize) {
    let mut samples = Vec::with_capacity(5);
    let mut bytes = 0;
    for _ in 0..5 {
        let start = Instant::now();
        let files = render_site_with_renderer(site, renderer).expect("renderer succeeds");
        samples.push(start.elapsed());
        bytes = files.iter().map(|file| file.contents.len()).sum();
    }
    samples.sort_unstable();
    (samples[2], bytes)
}
