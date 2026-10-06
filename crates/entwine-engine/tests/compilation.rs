use entwine_core::DiagnosticSeverity;
use entwine_engine::{compile, context_json, context_markdown, render_graph, render_site};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn kitchen() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/kitchen-sink")
}
fn write(root: &Path, name: &str, text: &str) {
    let path = root.join("docs").join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

#[test]
fn kitchen_sink_compiles_real_documents_and_all_projections() {
    let compilation = compile(&kitchen()).unwrap();
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics);
    assert_eq!(compilation.knowledge.documents.len(), 12);
    let auth = compilation
        .knowledge
        .documents
        .iter()
        .find(|d| d.id.0 == "specs/authentication.md")
        .unwrap();
    assert_eq!(auth.route.as_str(), "/specs/authentication/");
    assert_eq!(auth.metadata.kind.as_deref(), Some("spec"));
    assert_eq!(auth.metadata.status.as_deref(), Some("planned"));
    assert!(auth.html.contains("href=\"../../architecture/\""));
    assert!(auth.html.contains("href=\"../search/#query-contract\""));
    assert!(auth.headings.iter().any(|h| h.id == "session-boundary"));
    assert!(compilation.knowledge.backlinks(&auth.id).len() >= 3);
    assert_eq!(compilation.site.pages.len(), compilation.graph.nodes.len());
    assert_eq!(compilation.graph.edges, compilation.knowledge.relations);
    assert_eq!(
        compilation.context.relationships,
        compilation.knowledge.relations
    );
    let files = render_site(&compilation.site);
    assert!(files.iter().any(|f| f.path == "index.html"));
    let auth_html = String::from_utf8(
        files
            .iter()
            .find(|f| f.path == "specs/authentication/index.html")
            .unwrap()
            .contents
            .clone(),
    )
    .unwrap();
    for fragment in [
        "Referenced by",
        "Harbor architecture",
        "aria-current=\"page\"",
        "On this page",
        ">Knowledge</a>",
        ">Graph</a>",
        "<pre><code",
    ] {
        assert!(auth_html.contains(fragment), "missing {fragment}");
    }
    assert!(!auth_html.contains(".md\""));
    let architecture = compilation
        .knowledge
        .documents
        .iter()
        .find(|d| d.id.0 == "architecture.md")
        .unwrap();
    assert!(architecture.html.contains("<table>"));
    assert!(compilation.knowledge.documents[0].id.0 < compilation.knowledge.documents[1].id.0);
    let graph = String::from_utf8(render_graph(&compilation.graph).contents).unwrap();
    assert_eq!(graph.matches("data-node=").count(), 12);
    assert_eq!(
        graph.matches("data-source=").count(),
        compilation.graph.edges.len()
    );
    assert!(graph.contains("../../specs/authentication/"));
    let context: serde_json::Value =
        serde_json::from_str(&context_json(&compilation.context).unwrap()).unwrap();
    assert_eq!(context["schema_version"], "0.2");
    assert_eq!(context["documents"].as_array().unwrap().len(), 12);
    let markdown = context_markdown(&compilation.context);
    for part in [
        "# Project: Harbor",
        "## Documents",
        "## Relationships",
        "Backlinks:",
        "Type: spec",
        "Status: planned",
        "## Session boundary",
    ] {
        assert!(markdown.contains(part));
    }
}

#[test]
fn identical_input_produces_identical_output() {
    let first = compile(&kitchen()).unwrap();
    let second = compile(&kitchen()).unwrap();
    assert_eq!(first.knowledge, second.knowledge);
    assert_eq!(render_site(&first.site), render_site(&second.site));
    assert_eq!(render_graph(&first.graph), render_graph(&second.graph));
    assert_eq!(
        context_json(&first.context).unwrap(),
        context_json(&second.context).unwrap()
    );
    assert_eq!(
        context_markdown(&first.context),
        context_markdown(&second.context)
    );
}

#[test]
fn dedicated_broken_fixtures_produce_actionable_errors() {
    for (fixture, message) in [
        ("broken-link", "Broken internal link"),
        ("route-collision", "Route collision"),
        ("invalid-frontmatter", "Frontmatter title"),
    ] {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(fixture);
        let compilation = compile(&root).unwrap();
        assert!(compilation.has_errors());
        assert!(compilation
            .diagnostics
            .iter()
            .any(|d| d.message.contains(message)));
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/broken-link");
    assert_eq!(compile(&root).unwrap().diagnostics[0].line, Some(3));
}

#[test]
fn heading_ids_links_escaping_assets_and_relations_are_consistent() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), "index.md", "---\ntitle: '<Home & friends>'\n---\n# Home\n\n[One](nested/a%20b.md#repeat-2) [Again](nested/a%20b.md)\n\n<script>alert('x')</script>\n");
    write(
        temp.path(),
        "nested/a b.md",
        "# Title & <unsafe>\n\n## Repeat\n\n## Repeat\n\n![Asset](../picture.svg)\n\n[Home](/)\n",
    );
    fs::write(temp.path().join("docs/picture.svg"), "<svg></svg>").unwrap();
    let compilation = compile(temp.path()).unwrap();
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics);
    assert_eq!(compilation.knowledge.relations.len(), 2);
    assert_eq!(
        compilation
            .knowledge
            .backlinks(&compilation.knowledge.documents[1].id)
            .len(),
        1
    );
    let nested = &compilation.knowledge.documents[1];
    assert!(nested.html.contains("id=\"repeat-2\""));
    assert!(nested.html.contains("src=\"../../picture.svg\""));
    assert!(nested.html.contains("href=\"../../\""));
    assert!(compilation.knowledge.documents[0]
        .html
        .contains("nested/a%20b/#repeat-2"));
    let files = render_site(&compilation.site);
    let home = String::from_utf8(
        files
            .iter()
            .find(|f| f.path == "index.html")
            .unwrap()
            .contents
            .clone(),
    )
    .unwrap();
    assert!(home.contains("&lt;Home &amp; friends&gt;"));
    assert!(!home.contains("<script>"));
    assert!(home.contains("&lt;script&gt;"));
    let graph = String::from_utf8(render_graph(&compilation.graph).contents).unwrap();
    assert!(!graph.contains("<Home & friends>"));
}

#[test]
fn malformed_paths_metadata_anchors_and_schemes_fail_without_panics() {
    for (text, expected) in [
        ("# Home\n[x](../outside.md)", "outside docs/"),
        ("# Home\n[x](%2e%2e/outside.md)", "outside docs/"),
        ("# Home\n[x](javascript:alert)", "scheme"),
        ("# Home\n[x](#missing)", "heading anchor"),
        ("---\ntitle: [\n---\n# Home", "Invalid frontmatter"),
        ("---\ntitle: Home\n", "Unclosed frontmatter"),
        ("---\n- foo\n---\n# Home", "YAML mapping"),
    ] {
        let temp = tempfile::tempdir().unwrap();
        write(temp.path(), "index.md", text);
        let compilation = compile(temp.path()).unwrap();
        assert!(
            compilation
                .diagnostics
                .iter()
                .any(|d| d.severity == DiagnosticSeverity::Error && d.message.contains(expected)),
            "{text}: {:?}",
            compilation.diagnostics
        );
    }
}

#[test]
fn graph_sizes_and_filesystem_order_are_deterministic() {
    for count in [1, 5, 20, 100] {
        let temp = tempfile::tempdir().unwrap();
        for i in (0..count).rev() {
            let link = if i + 1 < count {
                format!("\n[Next](page-{:03}.md)\n", i + 1)
            } else {
                String::new()
            };
            write(
                temp.path(),
                &format!("page-{i:03}.md"),
                &format!("# Page {i}\n{link}"),
            );
        }
        let compilation = compile(temp.path()).unwrap();
        assert!(!compilation.has_errors());
        assert_eq!(compilation.graph.nodes.len(), count);
        assert_eq!(compilation.graph.edges.len(), count - 1);
        let graph = render_graph(&compilation.graph);
        assert_eq!(graph, render_graph(&compile(temp.path()).unwrap().graph));
        let graph = String::from_utf8(graph.contents).unwrap();
        assert_eq!(graph.matches("data-node=").count(), count);
        assert_eq!(graph.matches("data-source=").count(), count - 1);
    }
}

#[test]
fn missing_and_empty_docs_are_actionable() {
    let temp = tempfile::tempdir().unwrap();
    assert!(compile(temp.path())
        .unwrap_err()
        .to_string()
        .contains("Create docs/index.md"));
    fs::create_dir(temp.path().join("docs")).unwrap();
    assert!(compile(temp.path()).unwrap().has_errors());
}

#[test]
fn projects_without_index_get_a_useful_entry_page() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), "architecture.md", "# Architecture\n[Home](/)");
    let compilation = compile(temp.path()).unwrap();
    assert!(!compilation.has_errors());
    assert_eq!(compilation.knowledge.documents.len(), 1);
    assert_eq!(compilation.graph.nodes.len(), 1);
    let index = render_site(&compilation.site)
        .into_iter()
        .find(|f| f.path == "index.html")
        .unwrap();
    assert!(String::from_utf8(index.contents)
        .unwrap()
        .contains("href=\"architecture/\""));
}

#[test]
fn generated_file_and_asset_collisions_are_diagnostics() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), "index.md", "# Home");
    write(temp.path(), "index.html.md", "# Impossible directory");
    assert!(compile(temp.path())
        .unwrap()
        .diagnostics
        .iter()
        .any(|d| d.message.contains("file/directory collision")));
    fs::remove_file(temp.path().join("docs/index.html.md")).unwrap();
    fs::write(temp.path().join("docs/index.html"), "asset").unwrap();
    assert!(compile(temp.path())
        .unwrap()
        .diagnostics
        .iter()
        .any(|d| d.message.contains("Asset collides")));
}

#[test]
fn dot_filenames_cannot_generate_escaping_routes() {
    for filename in [".md", "..md", "...md", "nested/...md"] {
        let temp = tempfile::tempdir().unwrap();
        write(temp.path(), "index.md", "# Home");
        write(temp.path(), filename, "# Unsafe");
        assert!(compile(temp.path()).unwrap().has_errors());
    }
}

#[cfg(unix)]
#[test]
fn backslash_asset_names_cannot_become_traversal() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), "index.md", "# Home");
    fs::write(temp.path().join("docs/..\\escaped.txt"), "bad").unwrap();
    assert!(compile(temp.path()).unwrap().has_errors());
}

#[cfg(unix)]
#[test]
fn symlinks_cannot_escape_docs() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), "index.md", "# Home");
    fs::write(temp.path().join("secret.md"), "# Secret").unwrap();
    std::os::unix::fs::symlink(
        temp.path().join("secret.md"),
        temp.path().join("docs/leak.md"),
    )
    .unwrap();
    assert!(compile(temp.path()).unwrap().has_errors());
}

#[test]
fn self_documentation_is_a_healthy_consumer() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    assert!(!compile(&root).unwrap().has_errors());
}

#[test]
fn roles_come_from_recognized_type_then_canonical_path_and_keep_raw_metadata() {
    use entwine_core::{DiagnosticSeverity, KnowledgeRole::*};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    write(root, "index.md", "# P\n[a](architecture.md) [s](design/system.md) [d](decisions/one.md) [g](guide.md) [r](specs/index.md)");
    write(
        root,
        "architecture.md",
        "---\ntype: roadmap\n---\n# Conflict",
    );
    write(
        root,
        "design/system.md",
        "---\ntype: Architecture\n---\n# System",
    );
    write(root, "decisions/one.md", "# One");
    write(root, "specs/index.md", "# Specs landing");
    write(root, "guide.md", "---\ntype: guide\n---\n# Guide");
    let compilation = compile(root).unwrap();
    assert!(!compilation.has_errors());
    let role = |id: &str| {
        compilation
            .knowledge
            .documents
            .iter()
            .find(|d| d.id.0 == id)
            .unwrap()
    };
    assert_eq!(role("index.md").role, Project);
    assert_eq!(role("architecture.md").role, Roadmap);
    assert_eq!(role("design/system.md").role, Architecture);
    assert_eq!(role("decisions/one.md").role, Decision);
    assert_eq!(role("specs/index.md").role, Other);
    assert_eq!(role("guide.md").role, Other);
    assert_eq!(role("guide.md").metadata.kind.as_deref(), Some("guide"));
    let conflicts: Vec<_> = compilation
        .diagnostics
        .iter()
        .filter(|d| d.message.contains("canonical path"))
        .collect();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].severity, DiagnosticSeverity::Warning);
    assert_eq!(conflicts[0].source, "architecture.md");
}

#[test]
fn knowledge_overview_lists_present_and_missing_areas_without_calling_them_errors() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), "index.md", "# P\n[s](state.md)");
    write(temp.path(), "state.md", "# S\n[p](index.md)");
    let compilation = compile(temp.path()).unwrap();
    let page =
        String::from_utf8(entwine_engine::render_knowledge(&compilation.site).contents).unwrap();
    for part in [
        "Project: <a href=\"../../\">P</a>",
        "Current state: <a href=\"../../state/\">S</a>",
        "Architecture <span class=\"state-text\">not found</span>",
        "No decision documents found.",
        "No specification documents found.",
    ] {
        assert!(page.contains(part), "missing {part}\n{page}");
    }
    assert!(!page.to_lowercase().contains("error"));
    assert!(!page.contains("Other knowledge"));
}

/// Regression fixtures distilled from running Entwine on real repositories
/// (ForgeCMS, Flowview, Agentyx). They pin both what works and what is known to fail.
#[test]
fn patterns_found_in_real_repositories() {
    use entwine_core::KnowledgeRole::*;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    // Uppercase canonical names and numbered specs are recognized without renames.
    write(
        root,
        "README.md",
        "# Docs\n[a](ARCHITECTURE.md) [s](STATE.md) [r](ROADMAP.md) [x](specs/001-list-filter.md)",
    );
    write(root, "ARCHITECTURE.md", "# Architecture\n[s](STATE.md)");
    write(
        root,
        "STATE.md",
        "# STATE — Current status\n[r](ROADMAP.md)",
    );
    write(root, "ROADMAP.md", "# Roadmap\n[a](ARCHITECTURE.md)");
    write(
        root,
        "specs/001-list-filter.md",
        "# Spec 001\n[s](../STATE.md)",
    );
    let compilation = compile(root).unwrap();
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics);
    let roles: Vec<_> = compilation
        .knowledge
        .documents
        .iter()
        .map(|d| (d.id.0.as_str(), d.role))
        .collect();
    assert!(roles.contains(&("ARCHITECTURE.md", Architecture)));
    assert!(roles.contains(&("STATE.md", State)));
    assert!(roles.contains(&("ROADMAP.md", Roadmap)));
    assert!(roles.contains(&("specs/001-list-filter.md", Spec)));
    assert!(roles.contains(&("README.md", Other)));
}

#[test]
fn known_real_world_failures_have_actionable_line_accurate_errors() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        "index.md",
        "# Home\n\n[Outside](../CLAUDE.md)\n\nSee [specs](specs/).\n\n[Jump](#status-07)\n\n<a id=\"status-07\"></a>\n",
    );
    write(temp.path(), "specs/one.md", "# One\n[home](../index.md)");
    let compilation = compile(temp.path()).unwrap();
    let messages: Vec<_> = compilation
        .diagnostics
        .iter()
        .filter(|d| d.severity == entwine_core::DiagnosticSeverity::Error)
        .map(|d| (d.line, d.message.as_str()))
        .collect();
    assert!(
        messages.contains(&(Some(3), "Link traverses outside docs/")),
        "{messages:?}"
    );
    assert!(
        messages
            .iter()
            .any(|(l, m)| *l == Some(5) && m.contains("docs/specs")),
        "{messages:?}"
    );
    assert!(
        messages
            .iter()
            .any(|(l, m)| *l == Some(7) && m.contains("#status-07")),
        "{messages:?}"
    );
}
