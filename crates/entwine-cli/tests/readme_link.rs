//! `entwine setup --readme-link` end to end against real Git repositories.
use std::{fs, path::Path, process::Command};

fn entwine(args: &[&str], cwd: &Path) -> (bool, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_entwine"))
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    (
        output.status.success(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
    )
}
fn git(cwd: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
        .args([
            "-c",
            "commit.gpgsign=false",
            "-c",
            "init.defaultBranch=main",
        ])
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}");
    String::from_utf8_lossy(&output.stdout).into_owned()
}
fn repo(remote: &str, readme: Option<&str>) -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    git(root, &["init", "-q"]);
    if !remote.is_empty() {
        git(root, &["remote", "add", "origin", remote]);
    }
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join("docs/index.md"), "# Home\n").unwrap();
    if let Some(readme) = readme {
        fs::write(root.join("README.md"), readme).unwrap();
    }
    temp
}
fn readme(root: &Path) -> String {
    fs::read_to_string(root.join("README.md")).unwrap()
}
const ORIGINAL: &str = "# Strata\n\nServer Components.\n\n```sh\n# keep me\n```\n";

#[test]
fn omitting_the_option_never_touches_the_readme() {
    let temp = repo("https://github.com/Andersseen/Strata.git", Some(ORIGINAL));
    let (ok, out) = entwine(&["setup"], temp.path());
    assert!(ok, "{out}");
    assert_eq!(readme(temp.path()), ORIGINAL);
    assert!(!out.contains("README"));
}

#[test]
fn dry_run_shows_the_exact_change_and_writes_nothing() {
    let temp = repo("git@github.com:Andersseen/Strata.git", Some(ORIGINAL));
    let (ok, out) = entwine(&["setup", "--dry-run", "--readme-link"], temp.path());
    assert!(ok, "{out}");
    assert!(out.contains("+ <!-- entwine:docs-link:start -->"), "{out}");
    assert!(
        out.contains("+ **Documentation:** <https://andersseen.github.io/Strata/>"),
        "{out}"
    );
    assert!(
        out.contains("not verified") || out.contains("has not verified"),
        "{out}"
    );
    assert!(out.contains("No files changed."));
    assert_eq!(readme(temp.path()), ORIGINAL);
    assert!(!temp.path().join(".github").exists());
}

#[test]
fn applies_once_is_idempotent_and_preserves_the_rest_byte_for_byte() {
    let temp = repo("https://github.com/Andersseen/Strata", Some(ORIGINAL));
    let root = temp.path();
    let (ok, out) = entwine(&["setup", "--readme-link"], root);
    assert!(ok, "{out}");
    let first = readme(root);
    assert!(first.contains("**Documentation:** <https://andersseen.github.io/Strata/>"));
    assert!(first.starts_with("# Strata\n\n<!-- entwine:docs-link:start -->"));
    assert!(first.ends_with("Server Components.\n\n```sh\n# keep me\n```\n"));
    // The workflow now exists, so the second run takes the "already configured" path;
    // README processing must still run and change nothing.
    let (ok, out) = entwine(&["setup", "--readme-link"], root);
    assert!(ok, "{out}");
    assert!(out.contains("already configured"), "{out}");
    assert!(
        out.contains("already has the managed documentation link"),
        "{out}"
    );
    assert_eq!(readme(root), first);
    // A new --site-url updates only the managed block.
    let (ok, out) = entwine(
        &[
            "setup",
            "--readme-link",
            "--site-url",
            "https://docs.example.com",
        ],
        root,
    );
    assert!(ok, "{out}");
    assert_eq!(
        readme(root),
        first.replace(
            "https://andersseen.github.io/Strata/",
            "https://docs.example.com/"
        )
    );
    // Nothing was committed or staged.
    assert_eq!(git(root, &["rev-list", "--count", "--all"]).trim(), "0");
    assert!(git(root, &["diff", "--cached", "--name-only"]).is_empty());
    // No leftover temporary files.
    assert!(fs::read_dir(root)
        .unwrap()
        .flatten()
        .all(|e| !e.file_name().to_string_lossy().contains("entwine-")));
}

#[test]
fn strata_style_readme_is_recognized_without_duplication() {
    let strata = "# Strata\n\n## Documentation\n\nTechnical documentation lives in [`docs/`](./docs/README.md). Compiled by\n<https://andersseen.github.io/Strata/>.\n";
    let temp = repo("https://github.com/Andersseen/Strata.git", Some(strata));
    let (ok, out) = entwine(&["setup", "--readme-link"], temp.path());
    assert!(ok, "{out}");
    assert!(
        out.contains("already links https://andersseen.github.io/Strata/"),
        "{out}"
    );
    assert_eq!(readme(temp.path()), strata);
}

#[test]
fn urls_are_never_invented_for_other_hosts() {
    for remote in [
        "https://gitlab.com/group/project.git",
        "https://bitbucket.org/ws/repo.git",
        "https://git.example.com/team/repo.git",
        "",
    ] {
        let temp = repo(remote, Some(ORIGINAL));
        let (ok, out) = entwine(&["setup", "--readme-link"], temp.path());
        assert!(ok, "{remote}: {out}");
        assert_eq!(readme(temp.path()), ORIGINAL, "{remote}");
        assert!(out.contains("--site-url"), "{remote}: {out}");
    }
    // An explicit URL works even where nothing can be derived.
    let temp = repo("https://gitlab.com/group/project.git", Some(ORIGINAL));
    let (ok, out) = entwine(
        &[
            "setup",
            "--readme-link",
            "--site-url",
            "https://group.gitlab.io/project/",
        ],
        temp.path(),
    );
    assert!(ok, "{out}");
    assert!(readme(temp.path()).contains("<https://group.gitlab.io/project/>"));
}

#[test]
fn unsafe_input_is_refused_before_anything_is_written() {
    let temp = repo("https://github.com/a/b.git", Some(ORIGINAL));
    let root = temp.path();
    for url in [
        "http://example.com/",
        "javascript:alert(1)",
        "https://u:p@example.com/",
        "https://example.com/a b",
    ] {
        let (ok, out) = entwine(&["setup", "--readme-link", "--site-url", url], root);
        assert!(!ok, "{url}: {out}");
        assert!(!root.join(".github").exists(), "{url}: CI was written");
    }
    assert_eq!(readme(root), ORIGINAL);
    let (ok, _) = entwine(&["setup", "--site-url", "https://example.com/"], root);
    assert!(!ok, "--site-url requires --readme-link");
    for bad in [
        "# T\n<!-- entwine:docs-link:start -->\n",
        "# T\n<!-- entwine:docs-link:end -->\n<!-- entwine:docs-link:start -->\n",
        "# T\n<!-- entwine:docs-link:start -->\n<!-- entwine:docs-link:end -->\n<!-- entwine:docs-link:start -->\n<!-- entwine:docs-link:end -->\n",
    ] {
        fs::write(root.join("README.md"), bad).unwrap();
        let (ok, out) = entwine(&["setup", "--readme-link"], root);
        assert!(!ok, "{out}");
        assert!(out.contains("README.md is not changed"), "{out}");
        assert_eq!(readme(root), bad);
    }
}

#[cfg(unix)]
#[test]
fn a_symlinked_readme_is_refused() {
    let temp = repo("https://github.com/a/b.git", None);
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("real.md"), "# Real\n").unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("real.md"),
        temp.path().join("README.md"),
    )
    .unwrap();
    let (ok, out) = entwine(&["setup", "--readme-link"], temp.path());
    assert!(!ok, "{out}");
    assert!(out.contains("symbolic link"), "{out}");
    assert_eq!(
        fs::read_to_string(outside.path().join("real.md")).unwrap(),
        "# Real\n"
    );
}

#[test]
fn missing_readme_is_reported_not_created_and_workflows_never_edit_it() {
    let temp = repo("https://github.com/a/b.git", None);
    let (ok, out) = entwine(&["setup", "--readme-link"], temp.path());
    assert!(ok, "{out}");
    assert!(!temp.path().join("README.md").exists());
    let workflow = fs::read_to_string(temp.path().join(".github/workflows/entwine.yml")).unwrap();
    assert!(!workflow.contains("README") && !workflow.contains("readme-link"));
}

#[test]
fn ambiguous_github_remotes_do_not_guess() {
    let temp = repo("https://github.com/owner/project.git", Some(ORIGINAL));
    git(
        temp.path(),
        &[
            "remote",
            "add",
            "upstream",
            "https://github.com/someone-else/project.git",
        ],
    );
    let (ok, out) = entwine(&["setup", "--readme-link"], temp.path());
    assert!(ok, "{out}");
    assert_eq!(readme(temp.path()), ORIGINAL);
    assert!(out.contains("ambiguous"), "{out}");
}
