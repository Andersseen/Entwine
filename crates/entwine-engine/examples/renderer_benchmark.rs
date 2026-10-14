//! Measure page rendering only, using one already projected kitchen-sink SiteModel.
//! Reports total site render time (which includes the single template compilation),
//! that compilation on its own, and the resulting per-page cost.
//! Run with `cargo run --release -p entwine-engine --example renderer_benchmark`.

use entwine_core::{PageReference, Route};
use entwine_engine::{compile, render_site};
use flowview_compiler::{compile_static, StaticCompileOptions};
use std::{path::PathBuf, time::Instant};

fn main() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/kitchen-sink");
    let mut site = compile(&project).expect("kitchen-sink compiles").site;
    let prototype = site.pages[0].clone();

    println!("Pages | Total ms | Template compile ms | Per page us | HTML bytes");
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

        let (total, bytes) = measure(&site);
        let compile = measure_compile();
        let per_page = total.saturating_sub(compile).as_secs_f64() * 1e6 / count as f64;
        println!(
            "{count} | {:.2} | {:.2} | {per_page:.1} | {bytes}",
            total.as_secs_f64() * 1000.0,
            compile.as_secs_f64() * 1000.0,
        );
    }
}

fn measure_compile() -> std::time::Duration {
    let source = include_str!("../src/templates/page.flow");
    median(|| {
        let start = Instant::now();
        compile_static(source, StaticCompileOptions::default()).expect("template compiles");
        (start.elapsed(), 0)
    })
    .0
}

fn measure(site: &entwine_core::SiteModel) -> (std::time::Duration, usize) {
    median(|| {
        let start = Instant::now();
        let files = render_site(site).expect("renderer succeeds");
        let elapsed = start.elapsed();
        (elapsed, files.iter().map(|file| file.contents.len()).sum())
    })
}

fn median(mut run: impl FnMut() -> (std::time::Duration, usize)) -> (std::time::Duration, usize) {
    let mut samples: Vec<_> = (0..7).map(|_| run()).collect();
    samples.sort_unstable();
    samples[3]
}
