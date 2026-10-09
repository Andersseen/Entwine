//! Minimal regressions distilled from consumer repositories; no external CI inputs.
use entwine_core::{KnowledgeRole, RepositorySource};
use entwine_engine::{compile, compile_with_source, context_json, render};
use std::{fs, path::Path};

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

#[test]
fn entrypoint_precedence_and_nested_readmes_are_deliberate() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    write(
        root,
        "docs/README.md",
        "# Project\n[Nested](guides/README.md)",
    );
    write(
        root,
        "docs/guides/README.md",
        "# Guide\n[Home](../README.md)",
    );
    let first = compile(root).unwrap();
    assert!(!first.has_errors());
    assert_eq!(first.knowledge.documents[0].route.as_str(), "/");
    assert_eq!(first.knowledge.documents[0].role, KnowledgeRole::Project);
    assert_eq!(
        first.knowledge.documents[1].route.as_str(),
        "/guides/README/"
    );
    assert_eq!(first.knowledge.documents[1].role, KnowledgeRole::Other);
    write(root, "docs/index.md", "# Canonical\n[Readme](README.md)");
    let both = compile(root).unwrap();
    assert!(!both.has_errors(), "{:?}", both.diagnostics);
    assert_eq!(
        both.site
            .pages
            .iter()
            .filter(|p| p.route.as_str() == "/")
            .count(),
        1
    );
    let readme = &both.knowledge.documents[0];
    assert_eq!(readme.route.as_str(), "/README/");
    assert_eq!(readme.role, KnowledgeRole::Other);
}

#[test]
fn generated_directory_pages_have_no_canonical_document_or_invented_relation() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    write(
        root,
        "docs/index.md",
        "# Home\n[Specs](specs/) [Deep](specs/nested/)",
    );
    write(root, "docs/specs/nested/auth.md", "# Authentication");
    let c = compile(root).unwrap();
    assert!(!c.has_errors(), "{:?}", c.diagnostics);
    assert!(c.knowledge.relations.is_empty());
    assert_eq!(c.context.documents.len(), 2);
    for route in ["/specs/", "/specs/nested/"] {
        let page = c
            .site
            .pages
            .iter()
            .find(|p| p.route.as_str() == route)
            .unwrap();
        assert!(page.html.contains("Authentication"));
        assert!(page.source_path.is_none());
    }
    write(
        root,
        "docs/specs/index.md",
        "# Authored\nThis is the real introduction.",
    );
    let c = compile(root).unwrap();
    assert!(!c.has_errors());
    let pages: Vec<_> = c
        .site
        .pages
        .iter()
        .filter(|p| p.route.as_str() == "/specs/")
        .collect();
    assert_eq!(pages.len(), 1);
    assert!(pages[0].html.contains("real introduction"));
}

#[test]
fn repository_references_retain_source_and_never_become_document_relations() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    write(root, "AGENTS.md", "Instructions");
    write(root, "packages/core/a b.json", "{}");
    write(
        root,
        "docs/index.md",
        "# Home\n[Instructions](../AGENTS.md) [Spec](specs/auth.md)",
    );
    write(
        root,
        "docs/specs/auth.md",
        "# Auth\n[Package](../../packages/core/a%20b.json#field) [Home](../index.md)",
    );
    let c = compile(root).unwrap();
    assert!(!c.has_errors(), "{:?}", c.diagnostics);
    assert_eq!(c.knowledge.relations.len(), 2);
    assert_eq!(
        c.knowledge.documents[1].repository_references[0].path,
        "packages/core/a b.json"
    );
    assert!(c.knowledge.documents[1]
        .html
        .contains("repository-reference"));
    assert!(!c.knowledge.documents[1]
        .html
        .contains("href=\"../../packages"));
    let json = context_json(&c.context).unwrap();
    assert!(json.contains("repository_references") && json.contains("0.4"));
    let source = RepositorySource {
        files: None,
        file_base_url: "https://github.com/org/repo/blob/abc/".into(),
    };
    let c = compile_with_source(root, root, Some(&source)).unwrap();
    assert!(c.knowledge.documents[1]
        .html
        .contains("https://github.com/org/repo/blob/abc/packages/core/a%20b.json#field"));
    assert_eq!(
        c.site.pages[0].source_url.as_deref(),
        Some("https://github.com/org/repo/blob/abc/docs/index.md")
    );
    assert!(!render(&c)
        .unwrap()
        .iter()
        .any(|f| f.path.contains("a b.json")));
}

#[test]
fn nested_project_uses_an_explicit_repository_boundary() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), "AGENTS.md", "instructions");
    write(
        temp.path(),
        "apps/site/docs/index.md",
        "# Home\n[Root](../../../AGENTS.md)",
    );
    let project = temp.path().join("apps/site");
    assert!(compile(&project).unwrap().has_errors());
    let c = compile_with_source(&project, temp.path(), None).unwrap();
    assert!(!c.has_errors());
    assert_eq!(
        c.knowledge.documents[0].repository_references[0].path,
        "AGENTS.md"
    );
}

#[test]
fn traversal_and_encoded_paths_never_escape_the_repository() {
    for link in [
        "../../secret.md",
        "%2e%2e/%2e%2e/secret.md",
        "..%5csecret.md",
        "../C%3A/secret.md",
        "../%00secret.md",
    ] {
        let temp = tempfile::tempdir().unwrap();
        write(
            temp.path(),
            "docs/index.md",
            &format!("# Home\n[bad]({link})"),
        );
        let c = compile(temp.path()).unwrap();
        assert!(c.has_errors(), "{link}");
        assert!(c.knowledge.documents[0].repository_references.is_empty());
        assert_eq!(
            c.diagnostics
                .iter()
                .find(|d| d.line.is_some())
                .unwrap()
                .line,
            Some(2)
        );
    }
}

#[cfg(unix)]
#[test]
fn in_repository_and_escaping_symlinks_are_rejected() {
    for directory in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        write(temp.path(), "actual/file.md", "content");
        let (target, link, destination) = if directory {
            (
                temp.path().join("actual"),
                temp.path().join("alias"),
                "../alias/file.md",
            )
        } else {
            (
                temp.path().join("actual/file.md"),
                temp.path().join("alias.md"),
                "../alias.md",
            )
        };
        std::os::unix::fs::symlink(target, link).unwrap();
        write(
            temp.path(),
            "docs/index.md",
            &format!("# Home\n[x]({destination})"),
        );
        let c = compile(temp.path()).unwrap();
        assert!(c.has_errors());
        assert!(c
            .diagnostics
            .iter()
            .any(|d| d.message.contains("symbolic link")));
    }
}

#[test]
fn only_empty_safe_anchors_are_emitted_and_heading_collisions_get_suffixes() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), "docs/index.md", "# Home\n\n[Anchor](#custom) [Heading](#custom-2)\n\n<a id=\"custom\"></a>\n\n## Custom\n\n<a id=\"bad\" onclick=\"alert(1)\"></a>\n\n<script>alert(1)</script>\n\n<a id=\"x&amp;y\"></a>\n");
    let c = compile(temp.path()).unwrap();
    assert!(!c.has_errors(), "{:?}", c.diagnostics);
    let d = &c.knowledge.documents[0];
    assert_eq!(d.anchors, ["custom"]);
    assert!(d.html.contains("<span id=\"custom\"></span>"));
    assert!(d.html.contains("id=\"custom-2\""));
    assert!(!d.html.contains("<script>") && !d.html.contains("<a id=\"bad\""));
    write(
        temp.path(),
        "docs/index.md",
        "# Home\n\n<a id=\"main\"></a>",
    );
    assert!(compile(temp.path()).unwrap().has_errors());
    write(
        temp.path(),
        "docs/index.md",
        "# Home\n\n<a id=\"twice\"></a>\n\n<a id=\"twice\"></a>",
    );
    assert!(compile(temp.path()).unwrap().has_errors());
}

#[test]
fn source_metadata_is_safe_and_uncommitted_paths_degrade_gracefully() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), "new.md", "uncommitted");
    write(temp.path(), "docs/index.md", "# Home\n[New](../new.md)");
    for bad in [
        "javascript:alert(1)/",
        "https://user:secret@github.com/org/repo/blob/main/",
        "https://github.com/org/repo?token=secret/",
    ] {
        let source = RepositorySource {
            file_base_url: bad.into(),
            files: None,
        };
        assert!(compile_with_source(temp.path(), temp.path(), Some(&source)).is_err());
    }
    let source = RepositorySource {
        file_base_url: "https://github.com/org/repo/blob/abc/".into(),
        files: Some(Default::default()),
    };
    let c = compile_with_source(temp.path(), temp.path(), Some(&source)).unwrap();
    assert!(!c.has_errors());
    assert!(c.knowledge.documents[0]
        .html
        .contains("repository-reference"));
    assert!(!c.knowledge.documents[0].html.contains("https://github.com"));
    assert!(c.site.pages[0].source_url.is_none());
}

#[test]
fn generated_directory_case_collisions_are_rejected_on_case_sensitive_hosts() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), "docs/A/one.md", "# One");
    write(temp.path(), "docs/a/two.md", "# Two");
    if fs::read_dir(temp.path().join("docs")).unwrap().count() < 2 {
        return;
    }
    let c = compile(temp.path()).unwrap();
    assert!(c.has_errors());
    assert!(c
        .diagnostics
        .iter()
        .any(|d| d.message.contains("differ only by case")));
}

#[test]
fn long_navigation_stays_bounded_but_current_page_and_full_index_are_reachable() {
    let temp = tempfile::tempdir().unwrap();
    for i in 0..100 {
        write(
            temp.path(),
            &format!("docs/page-{i:03}.md"),
            &format!("# Page {i}"),
        );
    }
    let c = compile(temp.path()).unwrap();
    let files = render(&c).unwrap();
    let page = files
        .iter()
        .find(|f| f.path == "page-099/index.html")
        .unwrap();
    let html = String::from_utf8_lossy(&page.contents);
    assert!(html.contains("All documents in Knowledge"));
    assert!(html.contains("aria-current=\"page\""));
    let knowledge = files
        .iter()
        .find(|f| f.path == "__entwine/knowledge/index.html")
        .unwrap();
    let knowledge = String::from_utf8_lossy(&knowledge.contents);
    for i in 0..100 {
        assert!(knowledge.contains(&format!("Page {i}</a>")));
    }
    assert!(html.len() < 20_000);
}

#[test]
fn structured_context_preserves_full_source_including_unknown_frontmatter() {
    let temp = tempfile::tempdir().unwrap();
    let text =
        "---\ntype: architecture\ncustom: retained in source\n---\n# Design\n\nFull source.\n";
    write(temp.path(), "docs/index.md", text);
    let c = compile(temp.path()).unwrap();
    assert_eq!(c.context.documents[0].content, text);
    assert_eq!(c.context.documents[0].role, KnowledgeRole::Architecture);
    assert!(!c.knowledge.documents[0].html.contains("custom:"));
}

#[test]
fn authored_uppercase_route_wins_over_case_equivalent_generated_directory() {
    let temporary = tempfile::tempdir().unwrap();
    write(
        temporary.path(),
        "docs/index.md",
        "# Project\n[Roadmap](roadmap/)",
    );
    write(
        temporary.path(),
        "docs/ROADMAP.md",
        "# Roadmap\n[Plan](roadmap/v1/plan.md)",
    );
    write(temporary.path(), "docs/roadmap/v1/plan.md", "# Plan");
    let c = compile(temporary.path()).unwrap();
    assert!(!c.has_errors(), "{:?}", c.diagnostics);
    assert_eq!(
        c.site
            .pages
            .iter()
            .filter(|p| p.route.as_str().eq_ignore_ascii_case("/roadmap/"))
            .count(),
        1
    );
    assert!(c
        .knowledge
        .documents
        .iter()
        .find(|d| d.id.0 == "index.md")
        .unwrap()
        .html
        .contains("href=\"ROADMAP/\""));
    let home = render(&c)
        .unwrap()
        .into_iter()
        .find(|f| f.path == "index.html")
        .unwrap();
    let html = String::from_utf8(home.contents).unwrap();
    assert!(!html.contains("href=\"roadmap/\""));
    assert!(html.contains("href=\"ROADMAP/\""));
}
