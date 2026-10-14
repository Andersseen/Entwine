//! Contract tests for the Flowview page renderer. The files in `tests/baseline/` were
//! captured from the removed built-in renderer and pin the behavior Flowview must keep.

use entwine_core::{Navigation, PageReference, Route};
use entwine_engine::{
    compile, compile_configured, compile_with_config, load_config, parse_config, render,
    render_site,
};
use std::{collections::BTreeMap, fs, path::Path};
use tempfile::tempdir;

fn fixture(root: &Path) {
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
    fs::write(
        root.join("docs/custom.md"),
        "---\ntitle: 'Café <script>alert(1)</script>'\ntype: custom\nstatus: draft\n---\n\nPárrafo Unicode sin H1.\n",
    )
    .unwrap();
    fs::write(root.join("docs/empty.md"), "---\ntitle: Empty page\n---\n").unwrap();
}

fn page(files: &[entwine_engine::StaticFile], path: &str) -> String {
    let file = files.iter().find(|file| file.path == path).unwrap();
    String::from_utf8(file.contents.clone()).unwrap()
}

#[test]
fn renderer_setting_is_a_deprecated_alias_and_builtin_is_rejected() {
    assert!(parse_config("").unwrap().site.renderer.is_none());
    assert!(parse_config("[site]\nrenderer = 'flowview'\n")
        .unwrap()
        .site
        .renderer
        .is_some());

    let builtin = parse_config("[site]\nrenderer = 'builtin'\n").unwrap_err();
    let builtin = builtin.to_string();
    assert!(builtin.contains("no longer supported"), "{builtin}");
    assert!(builtin.contains("Delete the `renderer` key"), "{builtin}");

    let unknown = parse_config("[site]\nrenderer = 'unknown'\n").unwrap_err();
    assert!(unknown.to_string().contains("unknown"));
    assert!(parse_config("[site]\nrender = 'flowview'\n").is_err());
}

#[test]
fn legacy_flowview_setting_builds_with_a_deprecation_warning() {
    let temp = tempdir().unwrap();
    fixture(temp.path());
    fs::write(
        temp.path().join("entwine.toml"),
        "[site]\nrenderer = 'flowview'\n",
    )
    .unwrap();
    let compilation = compile_configured(temp.path()).unwrap();
    assert!(!compilation.has_errors());
    assert!(compilation
        .diagnostics
        .iter()
        .any(|d| d.source == "entwine.toml" && d.message.contains("deprecated")));
    assert!(render(&compilation).is_ok());

    fs::write(temp.path().join("entwine.toml"), "").unwrap();
    let clean = compile_configured(temp.path()).unwrap();
    assert!(!clean.diagnostics.iter().any(|d| d.source == "entwine.toml"));
}

#[test]
fn deep_routes_use_prepared_links_and_escape_hostile_values() {
    let temp = tempdir().unwrap();
    fixture(temp.path());
    let compilation = compile(temp.path()).unwrap();
    let files = render(&compilation).unwrap();
    let html = page(&files, "deep/page/index.html");

    assert!(html.contains("<a class=\"skip\" href=\"#main\">Skip to content</a>"));
    assert!(html.contains("<main id=\"main\" tabindex=\"-1\">"));
    assert!(html.contains("href=\"../../__entwine/knowledge/\">Knowledge</a>"));
    assert!(html.contains("href=\"../../__entwine/graph/?focus=deep%2Fpage.md\">Graph</a>"));
    assert!(html.contains("class=\"level-2\"><a href=\"#details\">Details</a>"));
    assert!(html.contains("Referenced by"));
    assert!(html.contains("<strong>trusted</strong>"));
    assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
    assert!(html.contains("&lt;img src=x onerror=alert(1)&gt;"));
    assert!(!html.contains("<script>alert(1)</script>"));
    assert!(!html.contains("<img src=x onerror=alert(1)>"));
}

#[test]
fn every_host_string_is_escaped_in_text_and_attribute_positions() {
    let temp = tempdir().unwrap();
    fixture(temp.path());
    let compilation = compile(temp.path()).unwrap();
    let hostile = "<img src=x onerror=alert(1)>";
    let attribute = "\" onmouseover=\"alert(1)\" x=\"";
    let mut site = compilation.site.clone();
    site.navigation[0].label = hostile.into();
    let page_model = site
        .pages
        .iter_mut()
        .find(|candidate| candidate.route.as_str() == "/deep/page/")
        .unwrap();
    page_model.source_path = Some(hostile.into());
    page_model.source_url = Some(format!("https://example.invalid/{attribute}"));
    page_model.metadata.kind = Some(attribute.into());
    page_model.headings[0].text = hostile.into();
    page_model.backlinks[0].title = hostile.into();
    page_model.references.push(PageReference {
        title: hostile.into(),
        route: Route::from_source("index.md").unwrap(),
    });
    let html = page(&render_site(&site).unwrap(), "deep/page/index.html");

    assert!(html.matches("&lt;img src=x onerror=alert(1)&gt;").count() >= 6);
    assert!(!html.contains(hostile));
    assert!(!html.contains("onmouseover=\"alert"), "{html}");
    assert!(html.contains("&quot; onmouseover=&quot;alert(1)&quot;"));
    assert!(!html.contains("@flowview/runtime"));
    assert!(!html.contains("<script"));
}

#[test]
fn rendering_is_deterministic() {
    let temp = tempdir().unwrap();
    fixture(temp.path());
    let compilation = compile(temp.path()).unwrap();
    assert_eq!(render(&compilation).unwrap(), render(&compilation).unwrap());
}

#[test]
fn optional_sections_and_large_lists_match_the_builtin_contract() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/kitchen-sink");
    let mut site = compile(&project).unwrap().site;
    let current = site.pages[0].route.clone();
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

    let files = render_site(&site).unwrap();
    let html = String::from_utf8_lossy(&files[1].contents);
    assert!(html.contains("<details><summary>15 documents</summary>"));
    assert_eq!(html.matches("<summary>15 documents</summary>").count(), 2);
    assert!(html.contains("All documents in Knowledge"));
    assert!(html.contains("Source:"));
    assert!(html.contains("?a=1&amp;b=2"));
    for page in files.iter().filter(|f| f.path.ends_with("index.html")) {
        let html = String::from_utf8_lossy(&page.contents);
        for landmark in ["class=\"skip\"", "<main id=\"main\"", "aria-label=\""] {
            assert!(html.contains(landmark), "{} missing {landmark}", page.path);
        }
    }
}

#[test]
fn custom_metadata_unicode_and_empty_pages_render() {
    let temp = tempdir().unwrap();
    fixture(temp.path());
    let compilation = compile(temp.path()).unwrap();
    let files = render(&compilation).unwrap();

    let custom = page(&files, "custom/index.html");
    for text in [
        "Café &lt;script&gt;alert(1)&lt;/script&gt;",
        "custom",
        "draft",
        "Párrafo Unicode sin H1.",
        "<h1>Café &lt;script&gt;alert(1)&lt;/script&gt;</h1>",
    ] {
        assert!(custom.contains(text), "missing {text:?} in {custom}");
    }
    assert!(!custom.contains("<script>alert(1)</script>"));

    let empty = page(&files, "empty/index.html");
    assert!(empty.contains("<main id=\"main\""));
    assert!(empty.contains("<article>"));
}

/// Whitespace and void-element syntax are the only intended differences from the
/// built-in renderer's output.
fn normalize(html: &str) -> String {
    let collapsed = html.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed
        .replace(" />", ">")
        .replace("/>", ">")
        .replace("> ", ">")
        .replace(" <", "<")
}

#[test]
fn projection_fixture_matches_the_builtin_baseline_pages() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let compilation = compile(&root.join("fixtures/projection-golden")).unwrap();
    let files = render(&compilation).unwrap();
    for (path, baseline) in [
        ("index.html", "builtin-index.html"),
        ("specs/index.html", "builtin-specs__index.html"),
    ] {
        let expected = fs::read_to_string(root.join("baseline").join(baseline))
            .unwrap()
            .replace("PACKAGE_VERSION", env!("CARGO_PKG_VERSION"));
        assert_eq!(
            normalize(&page(&files, path)),
            normalize(&expected),
            "{path}"
        );
    }
}

#[test]
fn kitchen_sink_links_and_landmarks_match_the_builtin_baseline() {
    let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/kitchen-sink");
    // The baseline was built with the repository root as the repository boundary.
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let config = load_config(&project).unwrap();
    let compilation = compile_with_config(&project, &repository, None, &config).unwrap();
    let files = render(&compilation).unwrap();
    let snapshot: BTreeMap<String, serde_json::Value> = serde_json::from_str(
        &fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/baseline/kitchen-sink-builtin-pages.json"),
        )
        .unwrap(),
    )
    .unwrap();

    let mut rendered: Vec<_> = files
        .iter()
        .filter(|f| f.path.ends_with("index.html") && !f.path.starts_with("__entwine"))
        .map(|f| f.path.as_str())
        .collect();
    rendered.sort_unstable();
    assert_eq!(
        rendered,
        snapshot.keys().map(String::as_str).collect::<Vec<_>>()
    );
    for (path, expected) in &snapshot {
        let html = page(&files, path);
        assert_eq!(
            extract(&html, "href"),
            strings(&expected["hrefs"]),
            "{path}"
        );
        assert_eq!(
            extract(&html, "aria-label"),
            strings(&expected["aria_labels"]),
            "{path}"
        );
    }
}

fn strings(value: &serde_json::Value) -> Vec<String> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect()
}

fn extract(html: &str, attribute: &str) -> Vec<String> {
    let marker = format!(" {attribute}=\"");
    html.split(marker.as_str())
        .skip(1)
        .filter_map(|rest| rest.split_once('"').map(|(value, _)| value.to_owned()))
        .filter(|value| !value.starts_with("https://github.com/Andersseen/Entwine/blob/"))
        .collect()
}
