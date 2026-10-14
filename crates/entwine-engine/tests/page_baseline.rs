//! Renderer-independent page-shell baseline, captured from the built-in renderer.
//! `tests/baseline/kitchen-sink-pages.json` records each kitchen-sink page's hrefs,
//! aria-labels and element sequence. Regenerate it with `ENTWINE_UPDATE_GOLDENS=1`
//! after a reviewed page-shell change. Any page renderer must reproduce it, so it
//! remains regression evidence if the built-in renderer is ever removed.

use entwine_core::{RendererKind, SiteModel};
use entwine_engine::{compile, render_site_with_renderer};
use std::{collections::BTreeMap, fs, path::Path};

mod common;
use common::{attribute_values, normalize};

fn kitchen_sink() -> SiteModel {
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/kitchen-sink");
    compile(&project).unwrap().site
}

fn elements(html: &str) -> String {
    html.split('<')
        .skip(1)
        .filter_map(|tag| {
            let name = tag
                .split(|c: char| c.is_whitespace() || c == '>' || c == '/')
                .next()?;
            (!name.is_empty() && !name.starts_with('!')).then(|| name.to_owned())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn page_summaries(renderer: RendererKind) -> BTreeMap<String, serde_json::Value> {
    render_site_with_renderer(&kitchen_sink(), renderer)
        .unwrap()
        .into_iter()
        .filter(|file| file.path.ends_with("index.html"))
        .map(|file| {
            let html = normalize(&String::from_utf8(file.contents).unwrap());
            let html = html.replace(env!("CARGO_PKG_VERSION"), "PACKAGE_VERSION");
            let summary = serde_json::json!({
                "hrefs": attribute_values(&html, "href"),
                "aria_labels": attribute_values(&html, "aria-label"),
                "elements": elements(&html),
            });
            (file.path, summary)
        })
        .collect()
}

fn baseline_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/baseline/kitchen-sink-pages.json")
}

#[test]
fn builtin_page_shells_match_the_recorded_baseline() {
    let current = serde_json::to_string_pretty(&page_summaries(RendererKind::Builtin)).unwrap();
    let path = baseline_path();
    if std::env::var_os("ENTWINE_UPDATE_GOLDENS").is_some() {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, format!("{current}\n")).unwrap();
    }
    assert_eq!(
        format!("{current}\n"),
        fs::read_to_string(&path).unwrap().replace("\r\n", "\n")
    );
}

#[cfg(feature = "flowview-renderer")]
#[test]
fn flowview_page_shells_match_the_builtin_baseline() {
    let current = serde_json::to_string_pretty(&page_summaries(RendererKind::Flowview)).unwrap();
    assert_eq!(
        format!("{current}\n"),
        fs::read_to_string(baseline_path())
            .unwrap()
            .replace("\r\n", "\n")
    );
}

#[cfg(feature = "flowview-renderer")]
#[test]
fn flowview_pages_are_semantically_equal_to_builtin_output() {
    let site = kitchen_sink();
    let builtin = render_site_with_renderer(&site, RendererKind::Builtin).unwrap();
    let flowview = render_site_with_renderer(&site, RendererKind::Flowview).unwrap();
    assert_eq!(builtin.len(), flowview.len());
    for (expected, actual) in builtin.iter().zip(&flowview) {
        assert_eq!(expected.path, actual.path);
        assert_eq!(
            normalize(&String::from_utf8_lossy(&expected.contents)),
            normalize(&String::from_utf8_lossy(&actual.contents)),
            "{}",
            actual.path
        );
    }
}

#[cfg(feature = "flowview-renderer")]
#[test]
fn flowview_reproduces_the_reviewed_page_goldens() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut compilation = compile(&root.join("fixtures/projection-golden")).unwrap();
    compilation.config.site.renderer = RendererKind::Flowview;
    let files = entwine_engine::render(&compilation).unwrap();
    for (path, golden) in [
        ("index.html", "index.html"),
        ("specs/index.html", "specs__index.html"),
    ] {
        let file = files.iter().find(|file| file.path == path).unwrap();
        let output = String::from_utf8(file.contents.clone())
            .unwrap()
            .replace(env!("CARGO_PKG_VERSION"), "PACKAGE_VERSION");
        let golden = fs::read_to_string(root.join("golden").join(golden)).unwrap();
        assert_eq!(normalize(&output), normalize(&golden), "{path}");
    }
}
