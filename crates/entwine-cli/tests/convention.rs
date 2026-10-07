//! Knowledge convention, `init`, and `setup`, exercised through the real binary.
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    process::{Command, Output},
};

fn entwine(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_entwine"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}
fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}
fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
fn write(root: &Path, name: &str, text: &str) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}
fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(status.status.success(), "git {args:?}");
}
fn repo(remote: Option<&str>) -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    git(temp.path(), &["init", "-q"]);
    if let Some(remote) = remote {
        git(temp.path(), &["remote", "add", "origin", remote]);
    }
    temp
}
/// Every file outside `.git`, for before/after comparisons.
fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.file_name().unwrap() == ".git" {
                continue;
            }
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                    fs::read(&path).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

// ── check: coverage and strictness ─────────────────────────────────────────────

#[test]
fn arbitrary_markdown_compiles_and_missing_areas_are_not_fatal() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), "docs/foo.md", "# Foo\n[bar](bar.md)");
    write(temp.path(), "docs/bar.md", "# Bar\n[foo](foo.md)");
    write(
        temp.path(),
        "docs/random-name.md",
        "# Random\n[foo](foo.md)",
    );
    let check = entwine(temp.path(), &["check"]);
    assert!(check.status.success(), "{}", stderr(&check));
    let text = stderr(&check);
    assert!(text.contains("⚠ Architecture"), "{text}");
    assert!(text.contains("✓ validation passed"));
    assert!(entwine(temp.path(), &["build"]).status.success());
}

#[test]
fn strict_knowledge_is_opt_in_and_enforces_all_recommended_areas() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), "docs/index.md", "# P\n[a](architecture.md)");
    write(temp.path(), "docs/architecture.md", "# A\n[i](index.md)");
    assert!(entwine(temp.path(), &["check"]).status.success());
    let strict = entwine(temp.path(), &["check", "--strict-knowledge"]);
    assert!(!strict.status.success());
    let text = stderr(&strict);
    assert!(
        text.contains("✗ Current state")
            && text.contains("recommended knowledge areas are missing")
    );

    write(temp.path(), "docs/state.md", "# S\n[i](index.md)");
    write(temp.path(), "docs/roadmap.md", "# R\n[i](index.md)");
    write(
        temp.path(),
        "docs/decisions/one.md",
        "# D\n[i](../index.md)",
    );
    write(temp.path(), "docs/specs/one.md", "# S\n[i](../index.md)");
    let strict = entwine(temp.path(), &["check", "--strict-knowledge"]);
    assert!(strict.status.success(), "{}", stderr(&strict));
    let text = stderr(&strict);
    assert!(text.contains("✓ Decisions\n  1 document\n"), "{text}");
    assert!(!text.contains('%'), "coverage must not be a percentage");
}

#[test]
fn custom_filename_with_recognized_type_counts_and_conflicts_warn() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        "docs/index.md",
        "# P\n[s](design/system.md) [x](architecture.md)",
    );
    write(
        temp.path(),
        "docs/design/system.md",
        "---\ntype: architecture\n---\n# System\n[i](../index.md)",
    );
    // Canonical path says architecture; the explicit type disagrees.
    write(
        temp.path(),
        "docs/architecture.md",
        "---\ntype: roadmap\n---\n# Odd\n[i](index.md)",
    );
    let check = entwine(temp.path(), &["check"]);
    assert!(check.status.success());
    let text = stderr(&check);
    assert!(text.contains("warning: docs/architecture.md"), "{text}");
    assert!(text.contains("conflicts with the canonical path"));
    assert!(
        text.contains("✓ Architecture\n  docs/design/system.md"),
        "{text}"
    );
    assert!(text.contains("✓ Roadmap\n  docs/architecture.md"), "{text}");
}

#[test]
fn context_json_exposes_roles_under_an_explicit_schema_version() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        "docs/index.md",
        "# P\n[a](architecture.md) [d](decisions/x.md) [g](guide.md)",
    );
    write(temp.path(), "docs/architecture.md", "# A\n[i](index.md)");
    write(
        temp.path(),
        "docs/decisions/x.md",
        "---\ntype: adr-ish\n---\n# D\n[i](../index.md)",
    );
    write(
        temp.path(),
        "docs/guide.md",
        "---\ntype: guide\n---\n# G\n[i](index.md)",
    );
    let out = entwine(temp.path(), &["context", "--json"]);
    assert!(out.status.success());
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["schema_version"], "0.4");
    let role = |id: &str| {
        json["documents"]
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["id"] == id)
            .map(|d| (d["role"].clone(), d["metadata"]["type"].clone()))
            .unwrap()
    };
    assert_eq!(role("index.md").0, "project");
    assert_eq!(role("architecture.md").0, "architecture");
    assert_eq!(
        role("decisions/x.md"),
        ("decision".into(), "adr-ish".into())
    );
    // The raw author-provided type survives; the derived role is separate.
    assert_eq!(role("guide.md"), ("other".into(), "guide".into()));
    assert_eq!(
        out.stdout,
        entwine(temp.path(), &["context", "--json"]).stdout
    );
}

#[test]
fn knowledge_overview_and_role_badges_are_generated() {
    let temp = tempfile::tempdir().unwrap();
    assert!(entwine(temp.path(), &["init", "--yes"]).status.success());
    write(
        temp.path(),
        "docs/decisions/one.md",
        "# One\n[i](../index.md)",
    );
    assert!(entwine(temp.path(), &["build"]).status.success());
    let page = fs::read_to_string(temp.path().join("dist/__entwine/knowledge/index.html")).unwrap();
    for part in [
        "Project knowledge",
        "Core",
        "Decisions",
        "1 document",
        "No specification documents found",
        "suggestions, not requirements",
    ] {
        assert!(page.contains(part), "missing {part}");
    }
    let architecture =
        fs::read_to_string(temp.path().join("dist/architecture/index.html")).unwrap();
    assert!(architecture.contains("role role-architecture"));
    assert!(architecture.contains(">Knowledge</a>") && architecture.contains(">Graph</a>"));
    let graph = fs::read_to_string(temp.path().join("dist/__entwine/graph/index.html")).unwrap();
    assert!(graph.contains("data-role=\"architecture\""));
}

// ── init ───────────────────────────────────────────────────────────────────────

#[test]
fn init_scaffolds_an_empty_repository_that_then_validates() {
    let temp = repo(None);
    let init = entwine(temp.path(), &["init", "--yes"]);
    assert!(init.status.success(), "{}", stderr(&init));
    let text = stdout(&init);
    for created in [
        "docs/index.md",
        "docs/architecture.md",
        "docs/state.md",
        "docs/roadmap.md",
        "docs/decisions/",
        "docs/specs/",
    ] {
        assert!(text.contains(&format!("✓ Created {created}")), "{text}");
    }
    assert!(text.contains("Existing files were not changed."));
    assert!(fs::read_to_string(temp.path().join("docs/state.md"))
        .unwrap()
        .contains("What works today?"));
    let check = entwine(temp.path(), &["check"]);
    assert!(check.status.success(), "{}", stderr(&check));
    let text = stderr(&check);
    // Landing pages do not fake decision or spec coverage, and no invented orphan warnings.
    assert!(text.contains("⚠ Decisions") && text.contains("⚠ Specifications"));
    assert!(!text.contains("warning:"), "{text}");
}

#[test]
fn init_is_idempotent_and_never_changes_existing_files() {
    let temp = repo(None);
    entwine(temp.path(), &["init", "--yes"]);
    let before = tree(temp.path());
    let again = entwine(temp.path(), &["init", "--yes"]);
    assert!(stdout(&again).contains("✓ already configured"));
    assert_eq!(before, tree(temp.path()));
}

#[test]
fn init_preserves_readme_and_partial_documentation() {
    let temp = repo(None);
    write(temp.path(), "README.md", "# Mine\n");
    write(
        temp.path(),
        "docs/index.md",
        "# My own entry\n\nHand-written.\n",
    );
    write(
        temp.path(),
        "docs/architecture.md",
        "# Architecture\n\nReal content.\n",
    );
    let before = tree(temp.path());
    let init = entwine(temp.path(), &["init", "--yes"]);
    assert!(init.status.success());
    let after = tree(temp.path());
    for (path, bytes) in &before {
        assert_eq!(&after[path], bytes, "{path} changed");
    }
    assert!(after.contains_key("docs/state.md") && after.contains_key("docs/roadmap.md"));
    assert!(!stdout(&init).contains("Created docs/index.md"));
}

#[test]
fn init_recognizes_custom_filenames_by_type_without_creating_duplicates() {
    let temp = repo(None);
    write(temp.path(), "docs/index.md", "# Home\n");
    write(
        temp.path(),
        "docs/design/system.md",
        "---\ntype: architecture\n---\n# System\n",
    );
    let init = entwine(temp.path(), &["init", "--yes"]);
    assert!(init.status.success());
    let text = stdout(&init);
    assert!(
        text.contains("✓ architecture.md docs/design/system.md"),
        "{text}"
    );
    assert!(!temp.path().join("docs/architecture.md").exists());
}

#[test]
fn init_links_the_new_entry_point_to_custom_architecture() {
    let temp = repo(None);
    write(
        temp.path(),
        "docs/design/system.md",
        "---\ntype: architecture\n---\n# System\n",
    );
    entwine(temp.path(), &["init", "--yes"]);
    let index = fs::read_to_string(temp.path().join("docs/index.md")).unwrap();
    assert!(index.contains("(design/system.md)"), "{index}");
    assert!(!index.contains("(architecture.md)"));
    assert!(entwine(temp.path(), &["check"]).status.success());
}

#[test]
fn init_without_yes_in_a_non_terminal_changes_nothing_and_dry_run_never_writes() {
    let temp = repo(None);
    let before = tree(temp.path());
    assert!(entwine(temp.path(), &["init"]).status.success());
    assert!(entwine(temp.path(), &["init", "--dry-run", "--yes"])
        .status
        .success());
    assert_eq!(before, tree(temp.path()));
}

#[test]
fn init_refuses_to_write_through_a_symlinked_docs_directory() {
    #[cfg(unix)]
    {
        let temp = repo(None);
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), temp.path().join("docs")).unwrap();
        assert!(!entwine(temp.path(), &["init", "--yes"]).status.success());
        assert!(fs::read_dir(outside.path()).unwrap().next().is_none());
    }
}

// ── setup ──────────────────────────────────────────────────────────────────────

fn yaml(text: &str) -> serde_yaml::Value {
    serde_yaml::from_str(text).unwrap_or_else(|e| panic!("invalid YAML: {e}\n{text}"))
}
fn read(root: &Path, name: &str) -> String {
    fs::read_to_string(root.join(name)).unwrap()
}

#[test]
fn provider_detection_covers_https_ssh_and_unknown_remotes() {
    for (remote, provider, file) in [
        (
            "https://github.com/foo/bar.git",
            "GitHub",
            ".github/workflows/entwine.yml",
        ),
        (
            "git@github.com:foo/bar.git",
            "GitHub",
            ".github/workflows/entwine.yml",
        ),
        (
            "https://gitlab.com/foo/bar.git",
            "GitLab",
            ".gitlab/ci/entwine.yml",
        ),
        (
            "git@gitlab.com:foo/bar.git",
            "GitLab",
            ".gitlab/ci/entwine.yml",
        ),
        (
            "https://bitbucket.org/foo/bar.git",
            "Bitbucket Cloud",
            "bitbucket-pipelines.yml",
        ),
        (
            "git@bitbucket.org:foo/bar.git",
            "Bitbucket Cloud",
            "bitbucket-pipelines.yml",
        ),
    ] {
        let temp = repo(Some(remote));
        let out = entwine(temp.path(), &["setup"]);
        assert!(out.status.success(), "{remote}: {}", stderr(&out));
        assert!(
            stdout(&out).contains(&format!("Provider: {provider}")),
            "{remote}"
        );
        assert!(temp.path().join(file).is_file(), "{remote}");
    }
}

#[test]
fn unknown_remote_and_non_git_directories_fall_back_without_failing() {
    for temp in [
        repo(Some("https://git.internal.example/foo/bar.git")),
        repo(None),
        tempfile::tempdir().unwrap(),
    ] {
        let before = tree(temp.path());
        let out = entwine(temp.path(), &["setup"]);
        assert!(out.status.success());
        let text = stdout(&out);
        assert!(
            text.contains("entwine build") && text.contains("Publish dist/"),
            "{text}"
        );
        assert!(!text.to_lowercase().contains("cloudflare"));
        assert_eq!(before, tree(temp.path()));
    }
}

#[test]
fn dry_run_describes_the_plan_and_changes_nothing() {
    for remote in [
        "https://github.com/a/b.git",
        "https://gitlab.com/a/b.git",
        "https://bitbucket.org/a/b.git",
    ] {
        let temp = repo(Some(remote));
        write(temp.path(), "docs/index.md", "# P\n");
        let before = tree(temp.path());
        let out = entwine(temp.path(), &["setup", "--dry-run"]);
        assert!(out.status.success());
        let text = stdout(&out);
        assert!(
            text.contains("Would create:")
                && text.contains("Triggers:")
                && text.contains("entwine check"),
            "{text}"
        );
        assert!(text.contains("No files changed."));
        assert_eq!(before, tree(temp.path()));
    }
}

#[test]
fn setup_is_idempotent_for_every_provider() {
    for remote in [
        "https://github.com/a/b.git",
        "https://gitlab.com/a/b.git",
        "https://bitbucket.org/a/b.git",
    ] {
        let temp = repo(Some(remote));
        assert!(entwine(temp.path(), &["setup"]).status.success());
        let before = tree(temp.path());
        let again = entwine(temp.path(), &["setup"]);
        assert!(
            stdout(&again).contains("✓ already configured"),
            "{remote}: {}",
            stdout(&again)
        );
        assert_eq!(before, tree(temp.path()), "{remote}");
    }
}

#[test]
fn github_workflow_is_valid_least_privilege_and_does_not_deploy_pull_requests() {
    let temp = repo(Some("git@github.com:a/b.git"));
    entwine(temp.path(), &["setup"]);
    let text = read(temp.path(), ".github/workflows/entwine.yml");
    let doc = yaml(&text);
    assert_eq!(doc["permissions"]["contents"], "read");
    assert_eq!(doc["permissions"].as_mapping().unwrap().len(), 1);
    assert!(doc["jobs"]["build"]["permissions"].is_null());
    assert_eq!(doc["jobs"]["deploy"]["permissions"]["pages"], "write");
    assert_eq!(doc["jobs"]["deploy"]["permissions"]["id-token"], "write");
    assert_eq!(
        doc["jobs"]["deploy"]["if"],
        "github.event_name != 'pull_request'"
    );
    assert_eq!(doc["jobs"]["deploy"]["environment"]["name"], "github-pages");
    for event in ["pull_request", "push"] {
        let paths = doc["on"][event]["paths"].as_sequence().unwrap();
        assert_eq!(paths[0], "docs/**");
        assert_eq!(paths[1], "**");
        assert_eq!(paths[2], ".github/workflows/entwine.yml");
    }
    for action in [
        "actions/checkout@v",
        "actions/setup-node@v",
        "actions/upload-pages-artifact@v",
        "actions/deploy-pages@v",
    ] {
        assert!(text.contains(action), "{action}");
    }
    assert!(text.contains("default_branch") && !text.contains("branches: [main]"));
    assert!(text.contains(&format!(
        "ENTWINE_VERSION: \"{}\"",
        env!("CARGO_PKG_VERSION")
    )));
    assert!(!text.contains("secrets.") && !text.contains("cloudflare"));
    assert!(text.contains("entwine check") && text.contains("entwine build"));
    assert!(!text.contains("__"), "unfilled placeholder");
}

#[test]
fn gitlab_ci_is_isolated_included_and_uses_the_default_branch_variable() {
    let temp = repo(Some("https://gitlab.com/a/b.git"));
    entwine(temp.path(), &["setup"]);
    let ci = read(temp.path(), ".gitlab/ci/entwine.yml");
    let doc = yaml(&ci);
    for job in ["entwine:check", "entwine:build", "entwine:pages"] {
        assert!(doc[job].is_mapping(), "{job}");
    }
    assert_eq!(doc["entwine:pages"]["pages"]["publish"], "dist");
    assert_eq!(doc["entwine:build"]["artifacts"]["paths"][0], "dist");
    assert_eq!(doc["entwine:pages"]["needs"][0], "entwine:build");
    assert!(ci.contains("$CI_DEFAULT_BRANCH") && ci.contains("merge_request_event"));
    assert!(!ci.contains("== \"main\"") && !ci.contains("secrets"));
    let root = read(temp.path(), ".gitlab-ci.yml");
    assert_eq!(yaml(&root)["include"][0]["local"], ".gitlab/ci/entwine.yml");
}

#[test]
fn gitlab_existing_ci_is_extended_only_when_safe() {
    let safe = repo(Some("https://gitlab.com/a/b.git"));
    write(
        safe.path(),
        ".gitlab-ci.yml",
        "lint:\n  script: [echo hi]\n",
    );
    entwine(safe.path(), &["setup"]);
    let merged = read(safe.path(), ".gitlab-ci.yml");
    assert!(merged.ends_with("lint:\n  script: [echo hi]\n"), "{merged}");
    assert_eq!(
        yaml(&merged)["include"][0]["local"],
        ".gitlab/ci/entwine.yml"
    );
    assert!(yaml(&merged)["lint"].is_mapping());

    for existing in [
        "include:\n  - remote: https://example.com/x.yml\n",
        "stages: [a]\n",
        "workflow:\n  rules: []\n",
        "---\nlint: {}\n",
    ] {
        let unsafe_repo = repo(Some("https://gitlab.com/a/b.git"));
        write(unsafe_repo.path(), ".gitlab-ci.yml", existing);
        let out = entwine(unsafe_repo.path(), &["setup"]);
        assert!(out.status.success());
        assert_eq!(read(unsafe_repo.path(), ".gitlab-ci.yml"), existing);
        assert!(
            stdout(&out).contains("add this to it yourself"),
            "{}",
            stdout(&out)
        );
        assert!(unsafe_repo.path().join(".gitlab/ci/entwine.yml").is_file());
    }
}

#[test]
fn self_hosted_gitlab_needs_evidence_or_an_explicit_provider() {
    let temp = repo(Some("https://git.company.example/team/docs.git"));
    let before = tree(temp.path());
    assert!(stdout(&entwine(temp.path(), &["setup"])).contains("--provider"));
    assert_eq!(before, tree(temp.path()));
    let forced = entwine(temp.path(), &["setup", "--provider", "gitlab"]);
    assert!(forced.status.success());
    assert!(temp.path().join(".gitlab/ci/entwine.yml").is_file());
    assert!(temp.path().join(".gitlab-ci.yml").is_file());
    let inferred = repo(Some("https://gitlab.company.example/team/docs.git"));
    entwine(inferred.path(), &["setup"]);
    assert!(inferred.path().join(".gitlab/ci/entwine.yml").is_file());
}

#[test]
fn bitbucket_pipeline_is_valid_stores_no_credentials_and_explains_the_one_time_steps() {
    let temp = repo(Some("git@bitbucket.org:a/b.git"));
    git(temp.path(), &["checkout", "-q", "-b", "trunk"]);
    let out = entwine(temp.path(), &["setup"]);
    let text = stdout(&out);
    for part in [
        "<workspace>.bitbucket.io",
        "ENTWINE_SITE_TOKEN",
        "public even if the repository is private",
        "15 minutes",
    ] {
        assert!(text.contains(part), "missing {part}\n{text}");
    }
    let file = read(temp.path(), "bitbucket-pipelines.yml");
    let doc = yaml(&file);
    assert!(doc["pipelines"]["pull-requests"]["**"].is_sequence());
    assert!(
        doc["pipelines"]["branches"]["trunk"].is_sequence(),
        "uses the detected branch"
    );
    assert!(file.contains("${ENTWINE_SITE_TOKEN:-}") && file.contains("GIT_ASKPASS"));
    assert!(!file.contains("x-token-auth:$") && !file.contains("https://x-token"));
}

#[test]
fn bitbucket_existing_pipelines_are_never_edited() {
    let temp = repo(Some("https://bitbucket.org/a/b.git"));
    write(
        temp.path(),
        "bitbucket-pipelines.yml",
        "pipelines:\n  default:\n    - step:\n        script: [echo]\n",
    );
    let before = read(temp.path(), "bitbucket-pipelines.yml");
    let out = entwine(temp.path(), &["setup"]);
    assert!(out.status.success());
    assert_eq!(read(temp.path(), "bitbucket-pipelines.yml"), before);
    let snippet = read(temp.path(), ".entwine/bitbucket-pipelines.snippet.yml");
    assert!(yaml(&snippet)["pipelines"]["pull-requests"].is_mapping());
    let again = entwine(temp.path(), &["setup"]);
    assert!(stdout(&again).contains("Manual step") || stdout(&again).contains("⚠"));
}

#[test]
fn existing_unrelated_workflow_is_left_alone() {
    let temp = repo(Some("https://github.com/a/b.git"));
    write(temp.path(), ".github/workflows/entwine.yml", "name: mine\n");
    let out = entwine(temp.path(), &["setup"]);
    assert!(out.status.success());
    assert_eq!(
        read(temp.path(), ".github/workflows/entwine.yml"),
        "name: mine\n"
    );
    assert!(stdout(&out).contains("not changed"));
}

#[test]
fn monorepo_projects_get_prefixed_paths_and_unsafe_names_are_refused() {
    let temp = repo(Some("https://github.com/a/b.git"));
    write(temp.path(), "apps/site/docs/index.md", "# P\n");
    assert!(entwine(temp.path(), &["setup", "apps/site"])
        .status
        .success());
    let text = read(temp.path(), ".github/workflows/entwine.yml");
    assert!(
        text.contains("\"apps/site/docs/**\"")
            && text.contains("entwine check apps/site")
            && text.contains("path: apps/site/dist"),
        "{text}"
    );
    yaml(&text);

    let weird = repo(Some("https://github.com/a/b.git"));
    write(weird.path(), "my docs;x/docs/index.md", "# P\n");
    let before = tree(weird.path());
    let out = entwine(weird.path(), &["setup", "my docs;x"]);
    assert!(!out.status.success());
    assert_eq!(before, tree(weird.path()));
}

#[test]
fn generated_files_match_golden_snapshots_and_are_deterministic() {
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden");
    for (remote, file, name) in [
        (
            "https://github.com/a/b.git",
            ".github/workflows/entwine.yml",
            "github.yml",
        ),
        (
            "https://gitlab.com/a/b.git",
            ".gitlab/ci/entwine.yml",
            "gitlab.yml",
        ),
        (
            "https://bitbucket.org/a/b.git",
            "bitbucket-pipelines.yml",
            "bitbucket.yml",
        ),
    ] {
        let one = repo(Some(remote));
        let two = repo(Some(remote));
        git(one.path(), &["checkout", "-q", "-b", "main"]);
        git(two.path(), &["checkout", "-q", "-b", "main"]);
        entwine(one.path(), &["setup"]);
        entwine(two.path(), &["setup"]);
        let raw = read(one.path(), file);
        assert_eq!(raw, read(two.path(), file));
        // Goldens must not change on every release: the pinned version is a placeholder.
        let text = raw.replace(env!("CARGO_PKG_VERSION"), "{{VERSION}}");
        assert_ne!(text, raw, "the CLI version must be pinned in generated CI");
        if std::env::var_os("ENTWINE_UPDATE_GOLDEN").is_some() {
            fs::write(golden.join(name), &text).unwrap();
        }
        assert_eq!(
            text,
            fs::read_to_string(golden.join(name)).unwrap(),
            "{name}; set ENTWINE_UPDATE_GOLDEN=1 to refresh"
        );
    }
}

#[test]
fn init_accepts_readme_entrypoint_without_creating_a_competing_index() {
    let temporary = tempfile::tempdir().unwrap();
    write(temporary.path(), "docs/README.md", "# Existing project\n");
    let output = entwine(temporary.path(), &["init", "--yes"]);
    assert!(output.status.success(), "{}", stderr(&output));
    assert!(!temporary.path().join("docs/index.md").exists());
    assert_eq!(
        fs::read_to_string(temporary.path().join("docs/README.md")).unwrap(),
        "# Existing project\n"
    );
    assert!(temporary.path().join("docs/architecture.md").is_file());
    let output = entwine(temporary.path(), &["check"]);
    assert!(output.status.success());
    assert!(stderr(&output).contains("docs/README.md"));
}
