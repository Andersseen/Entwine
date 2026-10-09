use entwine_core::{Navigation, PageReference, RendererKind, Route};
use entwine_engine::{compile, parse_config, render, render_selected, render_site_with_renderer};
use std::{fs, path::Path};
use tempfile::tempdir;

fn fixture(root: &Path, renderer: Option<&str>) {
    fs::create_dir_all(root.join("docs/deep")).unwrap();
    fs::write(
        root.join("docs/index.md"),
        "---\ntitle: Harbor\ntype: project\nstatus: draft\n---\n# Harbor\n\nA **trusted** body.\n\n[Deep page](deep/page.md)\n",
    )
    .unwrap();
    fs::write(
        root.join("docs/deep/page.md"),
        "---\ntitle: \'\"><script>alert(1)</script>\'\ntype: architecture\nstatus: \'<img src=x onerror=alert(1)>\'\n---\n# Deep page\n\n## Details\n\nA **trusted** body.\n",
    )
    .unwrap();
    if let Some(renderer) = renderer {
        fs::write(
            root.join("entwine.toml"),
            format!("[site]\nrenderer = \'{renderer}\'\n"),
        )
        .unwrap();
    }
}

#[test]
fn renderer_config_defaults_to_builtin_and_rejects_unknown_values() {
    assert_eq!(parse_config("").unwrap().site.renderer.as_str(), "builtin");
    assert_eq!(
        parse_config("[site]\nrenderer = 'flowview'\n")
            .unwrap()
            .site
            .renderer
            .as_str(),
        "flowview"
    );
    let error = parse_config("[site]\nrenderer = 'unknown'\n").unwrap_err();
    assert!(error.to_string().contains("unknown"));
    assert!(error.to_string().contains("flowview"));
}

#[test]
fn flowview_uses_prepared_deep_routes_and_escapes_hostile_values() {
    let temp = tempdir().unwrap();
    fixture(temp.path(), Some("flowview"));
    let compilation = compile(temp.path()).unwrap();
    let files = render_selected(&compilation).unwrap();
    let page = String::from_utf8(
        files
            .iter()
            .find(|file| file.path == "deep/page/index.html")
            .unwrap()
            .contents
            .clone(),
    )
    .unwrap();

    assert!(page.contains("<a class=\"skip\" href=\"#main\">Skip to content</a>"));
    assert!(page.contains("<main id=\"main\" tabindex=\"-1\">"));
    assert!(page.contains("href=\"../../__entwine/knowledge/\">Knowledge</a>"));
    assert!(page.contains("href=\"../../__entwine/graph/?focus=deep%2Fpage.md\">Graph</a>"));
    assert!(page.contains("class=\"level-2\"><a href=\"#details\">Details</a>"));
    assert!(page.contains("Referenced by"));
    assert!(page.contains("<strong>trusted</strong>"));
    assert!(page.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
    assert!(page.contains("&lt;img src=x onerror=alert(1)&gt;"));
    assert!(!page.contains("<script>alert(1)</script>"));
    assert!(!page.contains("<img src=x onerror=alert(1)>"));

    let hostile = "<img src=x onerror=alert(1)>";
    let mut site = compilation.site.clone();
    let page_model = site
        .pages
        .iter_mut()
        .find(|candidate| candidate.route.as_str() == "/deep/page/")
        .unwrap();
    page_model.source_path = Some(hostile.into());
    page_model.backlinks[0].title = hostile.into();
    page_model.references.push(PageReference {
        title: hostile.into(),
        route: Route::from_source("index.md").unwrap(),
    });
    let hostile_page = render_site_with_renderer(&site, RendererKind::Flowview).unwrap();
    let hostile_page = String::from_utf8(
        hostile_page
            .iter()
            .find(|file| file.path == "deep/page/index.html")
            .unwrap()
            .contents
            .clone(),
    )
    .unwrap();
    assert!(
        hostile_page
            .matches("&lt;img src=x onerror=alert(1)&gt;")
            .count()
            >= 3
    );
    assert!(!hostile_page.contains(hostile));
}

#[test]
fn default_selection_is_the_builtin_baseline_and_flowview_is_deterministic() {
    let temp = tempdir().unwrap();
    fixture(temp.path(), None);
    let compilation = compile(temp.path()).unwrap();
    assert_eq!(
        render(&compilation).unwrap(),
        render_selected(&compilation).unwrap()
    );
    let first = render_selected(&compilation).unwrap();
    let second = render_selected(&compilation).unwrap();
    assert_eq!(first, second);
}

#[test]
fn both_renderers_keep_page_routes_and_link_targets_in_sync() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/kitchen-sink");
    let compilation = compile(&project).unwrap();
    let mut site = compilation.site.clone();
    let current = site.pages[0].route.clone();

    // Exercise the larger-list and optional-section paths on the same model.
    site.navigation.extend((0..40).map(|index| Navigation {
        label: format!("Extra page {index}"),
        route: Some(Route::from_source(&format!("extra/page-{index}.md")).unwrap()),
        children: Vec::new(),
    }));
    site.pages[0].source_url = Some("https://example.invalid/source.md?a=1&b=2".into());
    site.pages[0].backlinks = (0..15)
        .map(|index| PageReference {
            title: format!("Backlink {index}"),
            route: current.clone(),
        })
        .collect();
    site.pages[0].references = (0..15)
        .map(|index| PageReference {
            title: format!("Reference {index}"),
            route: Route::from_source(&format!("refs/item-{index}.md")).unwrap(),
        })
        .collect();

    let builtin = render_site_with_renderer(&site, RendererKind::Builtin).unwrap();
    let flowview = render_site_with_renderer(&site, RendererKind::Flowview).unwrap();
    assert_eq!(
        builtin.iter().map(|file| &file.path).collect::<Vec<_>>(),
        flowview.iter().map(|file| &file.path).collect::<Vec<_>>()
    );
    for (expected, actual) in builtin.iter().zip(&flowview) {
        if !expected.path.ends_with("index.html") {
            continue;
        }
        let expected = String::from_utf8_lossy(&expected.contents);
        let actual = String::from_utf8_lossy(&actual.contents);
        assert_eq!(href_targets(&expected), href_targets(&actual), "{}", actual);
        for landmark in ["class=\"skip\"", "<main id=\"main\"", "aria-label=\""] {
            assert!(actual.contains(landmark), "{} missing {landmark}", actual);
        }
        assert!(!actual.contains("@flowview/runtime"));
    }
    let source = String::from_utf8_lossy(&flowview[1].contents);
    assert!(source.contains("Source:"));
    assert!(source.contains("?a=1&amp;b=2"));
}

fn href_targets(html: &str) -> Vec<String> {
    html.split("href=\"")
        .skip(1)
        .filter_map(|attribute| attribute.split_once('\"').map(|(href, _)| href.to_owned()))
        .collect()
}
