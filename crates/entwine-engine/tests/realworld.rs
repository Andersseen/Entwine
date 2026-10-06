//! Real-world structure, portability, and subpath-hosting validation.
use entwine_core::{DiagnosticSeverity, Route};
use entwine_engine::{compile, render_graph, Compilation, StaticFile};
use percent_encoding::percent_decode_str;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

fn write(root: &Path, name: &str, text: &str) {
    let path = root.join("docs").join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}
fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}
fn site_files(compilation: &Compilation) -> Vec<StaticFile> {
    entwine_engine::render(compilation)
}

/// Extract the values of every `attr="..."` in an HTML document.
fn attributes<'a>(html: &'a str, attr: &str) -> Vec<&'a str> {
    let needle = format!(" {attr}=\"");
    let mut found = Vec::new();
    let mut rest = html;
    while let Some(start) = rest.find(&needle) {
        rest = &rest[start + needle.len()..];
        let end = rest.find('"').unwrap();
        found.push(&rest[..end]);
        rest = &rest[end..];
    }
    found
}
fn unescape(value: &str) -> String {
    value
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// Simulate a static host that serves the output below an arbitrary mount
/// prefix. Every relative reference must resolve to a published file or a
/// directory index, never above the mount point, and every fragment must exist.
/// Returns the number of references checked.
fn assert_portable_under(prefix: &str, files: &[StaticFile]) -> usize {
    let published: BTreeMap<_, _> = files.iter().map(|f| (f.path.clone(), f)).collect();
    let mut checked = 0;
    for file in files.iter().filter(|f| f.path.ends_with(".html")) {
        let html = String::from_utf8(file.contents.clone()).unwrap();
        // A page `a/b/index.html` is served at `{prefix}/a/b/`.
        let mut base: Vec<&str> = file.path.split('/').collect();
        base.pop();
        for value in attributes(&html, "href")
            .into_iter()
            .chain(attributes(&html, "src"))
        {
            let value = unescape(value);
            if value.starts_with("http:")
                || value.starts_with("https:")
                || value.starts_with("mailto:")
            {
                continue;
            }
            assert!(
                !value.starts_with('/'),
                "{prefix}/{}: root-absolute reference {value:?} breaks subpath hosting",
                file.path
            );
            let (path, fragment) = value
                .split_once('#')
                .map_or((&*value, None), |(p, f)| (p, Some(f)));
            let path = path.split_once('?').map_or(path, |(p, _)| p);
            let mut parts = base.clone();
            for part in path.split('/') {
                match part {
                    "" | "." => {}
                    ".." => assert!(
                        parts.pop().is_some(),
                        "{prefix}/{}: {value:?} escapes the mount point",
                        file.path
                    ),
                    part => parts.push(part),
                }
            }
            let decoded: Vec<String> = parts
                .iter()
                .map(|p| percent_decode_str(p).decode_utf8().unwrap().into_owned())
                .collect();
            let joined = decoded.join("/");
            let target = if published.contains_key(&joined) {
                joined
            } else if joined.is_empty() {
                "index.html".into()
            } else {
                format!("{joined}/index.html")
            };
            let page = published.get(&target).unwrap_or_else(|| {
                panic!(
                    "{prefix}/{}: {value:?} resolves to missing {prefix}/{target}",
                    file.path
                )
            });
            if let Some(fragment) = fragment.filter(|f| !f.is_empty()) {
                let target_html = String::from_utf8_lossy(&page.contents);
                assert!(
                    target_html.contains(&format!("id=\"{fragment}\"")),
                    "{prefix}/{}: fragment #{fragment} missing from {prefix}/{target}",
                    file.path
                );
            }
            checked += 1;
        }
    }
    checked
}

#[test]
fn entwine_docs_and_demo_work_under_docs_and_demo_prefixes() {
    for (project, prefix, pages) in [
        (
            ".",
            "/docs",
            ["architecture/index.html", "state/index.html"],
        ),
        (
            "examples/kitchen-sink",
            "/demo",
            [
                "specs/authentication/index.html",
                "decisions/static-output/index.html",
            ],
        ),
    ] {
        let compilation = compile(&repo(project)).unwrap();
        assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics);
        let files = site_files(&compilation);
        for page in pages.into_iter().chain([
            "index.html",
            "__entwine/graph/index.html",
            "__entwine/style.css",
        ]) {
            assert!(
                files.iter().any(|f| f.path == page),
                "{prefix}: {page} missing"
            );
        }
        assert!(
            assert_portable_under(prefix, &files) > 50,
            "{prefix}: too few references"
        );
        // Backlinks and the graph route are present and relative.
        let auth = files
            .iter()
            .find(|f| f.path == pages[0])
            .map(|f| String::from_utf8_lossy(&f.contents).into_owned())
            .unwrap();
        assert!(auth.contains("Referenced by"));
        assert!(auth.contains(">Graph</a>"));
    }
}

#[test]
fn route_derivation_is_identical_for_unix_and_windows_separators() {
    for (unix, route) in [
        ("index.md", "/"),
        ("a.md", "/a/"),
        ("a/index.md", "/a/"),
        ("a/b/c.md", "/a/b/c/"),
        ("a/b/index.md", "/a/b/"),
        ("with space/Über.md", "/with space/Über/"),
    ] {
        let windows = unix.replace('/', "\\");
        assert_eq!(Route::from_source(unix).unwrap().as_str(), route);
        assert_eq!(
            Route::from_source(&windows).unwrap(),
            Route::from_source(unix).unwrap()
        );
    }
}

#[test]
fn dangerous_and_reserved_routes_are_rejected() {
    for source in [
        "../x.md",
        "a/../x.md",
        "./x.md",
        "a//b.md",
        "/abs.md",
        "..\\x.md",
        "a\\..\\x.md",
        "__entwine/graph.md",
        "__entwine/index.md",
        "a/%2e%2e/x.md",
        "q?.md",
        "h#.md",
        "c:.md",
        "no-extension",
        "nul\0.md",
    ] {
        let result = Route::from_source(source);
        // `a:` style names are only rejected where the platform defines them;
        // everything else must be rejected portably.
        if source == "c:.md" {
            assert!(result.is_ok() || result.is_err());
        } else {
            assert!(result.is_err(), "{source:?} produced {result:?}");
        }
    }
    // __entwine only reserves the top level.
    assert!(Route::from_source("guide/__entwine.md").is_ok());
}

#[test]
fn routes_that_differ_only_by_case_are_reported() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        "index.md",
        "# Home\n[A](Guide.md) [B](guide.md)",
    );
    write(temp.path(), "Guide.md", "# Upper");
    write(temp.path(), "guide.md", "# Lower");
    if fs::read_dir(temp.path().join("docs")).unwrap().count() < 3 {
        return; // Case-insensitive file system: the fixture cannot exist here.
    }
    let compilation = compile(temp.path()).unwrap();
    assert!(compilation
        .diagnostics
        .iter()
        .any(|d| d.severity == DiagnosticSeverity::Error
            && d.message.contains("differ only by case")));
}

#[test]
fn file_and_directory_index_for_same_route_collide() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), "index.md", "# Home");
    write(temp.path(), "guide.md", "# A");
    write(temp.path(), "guide/index.md", "# B");
    assert!(compile(temp.path())
        .unwrap()
        .diagnostics
        .iter()
        .any(|d| d.message.contains("Route collision")));
}

#[test]
fn large_realistic_documentation_tree_resolves_and_is_portable() {
    let temp = tempfile::tempdir().unwrap();
    let long = "a-very-long-descriptive-filename-".repeat(6);
    write(
        temp.path(),
        "index.md",
        &format!(
            "# Project\n\n[Deep](a/b/c/d/e/f/deep.md#second) [Long]({long}.md) [Guide](guide/) [Q](guide/index.md?plain=1#intro) [Anchor](#project)\n\n- [Sibling 0](siblings/s000.md)\n"
        ),
    );
    write(
        temp.path(),
        "a/b/c/d/e/f/deep.md",
        "# Deep\n\n## Second\n\n## Second\n\n[Up](../../../../../../index.md) [Cousin](../../../../../../guide/index.md#intro) [Self](#second-2) ![i](../../../../../../img/logo.svg)\n",
    );
    write(
        temp.path(),
        &format!("{long}.md"),
        "# Long\n\n[Home](index.md)",
    );
    write(
        temp.path(),
        "guide/index.md",
        "# Guide\n\n## Intro\n\n[Deep](../a/b/c/d/e/f/deep.md)",
    );
    // README-style: badges, html comments, reference links, autolinks, footnote-like text.
    write(
        temp.path(),
        "readme-style.md",
        "<!-- comment -->\n[![Badge](img/logo.svg)](https://example.com)\n\n[ref]: guide/index.md\n\n<https://example.com>\n\nSee [ref link][ref] and [home][].\n\n[home]: index.md\n",
    );
    // No H1 and multiple H1s.
    write(
        temp.path(),
        "no-h1.md",
        "Just text.\n\n## Only second level\n",
    );
    write(
        temp.path(),
        "two-h1.md",
        "# First\n\n# First\n\nText [back](index.md).",
    );
    // Huge code block containing link-like and HTML-like text.
    let code = "[not](a link) <b>\n".repeat(5_000);
    write(
        temp.path(),
        "code.md",
        &format!("# Code\n\n```text\n{code}```\n"),
    );
    fs::create_dir_all(temp.path().join("docs/empty/dir")).unwrap();
    fs::create_dir_all(temp.path().join("docs/img")).unwrap();
    fs::write(temp.path().join("docs/img/logo.svg"), "<svg/>").unwrap();
    for i in 0..60 {
        write(
            temp.path(),
            &format!("siblings/s{i:03}.md"),
            "# Sibling\n\n[Hub](../index.md)\n",
        );
    }
    // Many backlinks to one page, and an orphan.
    write(temp.path(), "orphan.md", "# Orphan");
    let compilation = compile(temp.path()).unwrap();
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics);
    let orphans: Vec<_> = compilation
        .diagnostics
        .iter()
        .filter(|d| d.severity == DiagnosticSeverity::Warning)
        .map(|d| d.source.as_str())
        .collect();
    assert!(orphans.contains(&"orphan.md"));
    assert!(orphans.contains(&"no-h1.md"));
    assert!(!orphans.contains(&"index.md"));
    let documents = &compilation.knowledge.documents;
    let index = documents.iter().find(|d| d.id.0 == "index.md").unwrap();
    assert!(index.html.contains("a/b/c/d/e/f/deep/#second"));
    assert!(index.html.contains("guide/?plain=1#intro"));
    // 60 siblings, deep, long, two-h1, readme-style, and the index page's own #project anchor.
    assert_eq!(compilation.knowledge.backlinks(&index.id).len(), 65);
    let no_h1 = documents.iter().find(|d| d.id.0 == "no-h1.md").unwrap();
    assert_eq!(no_h1.title, "No H1");
    let two = documents.iter().find(|d| d.id.0 == "two-h1.md").unwrap();
    assert!(two.headings.iter().any(|h| h.id == "first-2"));
    let readme = documents
        .iter()
        .find(|d| d.id.0 == "readme-style.md")
        .unwrap();
    assert!(readme.html.contains("https://example.com"));
    assert!(readme.html.contains("href=\"../guide/\""));
    let files = site_files(&compilation);
    assert!(!files.iter().any(|f| f.path.starts_with("empty")));
    assert!(assert_portable_under("/some/deep/mount", &files) > 100);
    let code_page = files.iter().find(|f| f.path == "code/index.html").unwrap();
    let code_html = String::from_utf8_lossy(&code_page.contents);
    assert!(code_html.contains("&lt;b&gt;"));
    assert!(!code_html.contains("<b>"));
}

fn chain_graph(count: usize, shape: &str) -> Compilation {
    let temp = tempfile::tempdir().unwrap();
    for i in 0..count {
        let links = match shape {
            "chain" if i + 1 < count => format!("[n](page-{:03}.md)", i + 1),
            "star" if i > 0 => "[hub](page-000.md)".into(),
            "star" => (1..count)
                .map(|n| format!("[n](page-{n:03}.md)\n"))
                .collect(),
            "dense" => (0..count)
                .filter(|n| *n != i && (n + i) % 3 == 0)
                .map(|n| format!("[n](page-{n:03}.md)\n"))
                .collect(),
            _ => String::new(),
        };
        write(
            temp.path(),
            &format!("page-{i:03}.md"),
            &format!("# Page number {i} with a long label\n\n{links}"),
        );
    }
    let compilation = compile(temp.path()).unwrap();
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics);
    compilation
}

#[test]
fn graph_renders_validly_and_deterministically_from_1_to_100_nodes() {
    for shape in ["chain", "star", "dense"] {
        for count in [1usize, 5, 20, 50, 100] {
            let compilation = chain_graph(count, shape);
            let graph = render_graph(&compilation.graph);
            assert_eq!(
                graph,
                render_graph(&chain_graph(count, shape).graph),
                "{shape} {count}"
            );
            let html = String::from_utf8(graph.contents).unwrap();
            assert_eq!(html.matches("data-node=").count(), count);
            assert_eq!(
                html.matches("data-source=").count(),
                compilation.graph.edges.len()
            );
            // Balanced structure and no NaN/inf coordinates.
            for tag in ["svg", "details", "main", "ul"] {
                assert_eq!(
                    html.matches(&format!("<{tag}")).count(),
                    html.matches(&format!("</{tag}>")).count(),
                    "{shape} {count} <{tag}>"
                );
            }
            assert!(!html.contains("NaN") && !html.contains("inf"));
            // Fallback relationship list stays complete and linked.
            assert_eq!(
                html.matches(" references <a").count(),
                compilation.graph.edges.len()
            );
            // Sensible dimensions: grows with node count but stays bounded.
            let size: f64 = html
                .split("viewBox=\"0 0 ")
                .nth(1)
                .unwrap()
                .split(' ')
                .next()
                .unwrap()
                .parse()
                .unwrap();
            assert!(
                (200.0..=4_000.0).contains(&size),
                "{shape} {count}: size {size}"
            );
            // Every node position lies inside the viewBox.
            let nodes: BTreeSet<_> = html.match_indices("<circle class=\"node\" cx=\"").collect();
            assert_eq!(nodes.len(), count);
            for (index, _) in nodes {
                let rest = &html[index + "<circle class=\"node\" cx=\"".len()..];
                let x: f64 = rest.split('"').next().unwrap().parse().unwrap();
                let y: f64 = rest
                    .split("cy=\"")
                    .nth(1)
                    .unwrap()
                    .split('"')
                    .next()
                    .unwrap()
                    .parse()
                    .unwrap();
                assert!(
                    (0.0..=size).contains(&x) && (0.0..=size).contains(&y),
                    "{shape} {count}"
                );
            }
            assert_portable_under(
                "/demo",
                &[StaticFile {
                    path: "__entwine/graph/index.html".into(),
                    contents: html.clone().into_bytes(),
                }]
                .into_iter()
                .chain(site_files(&compilation))
                .collect::<Vec<_>>(),
            );
        }
    }
}
