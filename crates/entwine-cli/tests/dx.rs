//! End-user CLI journeys and developer-experience regressions found by dogfooding.
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn entwine(project: &Path, args: &[&str]) -> Output {
    // An empty global Git config keeps a developer's own ignore rules out of the tests.
    let config = std::env::temp_dir().join("entwine-dx-empty-gitconfig");
    let _ = fs::write(&config, "");
    Command::new(env!("CARGO_BIN_EXE_entwine"))
        .args(args)
        .current_dir(project)
        .env("GIT_CONFIG_GLOBAL", &config)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap()
}
fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}
fn git_init(path: &Path) {
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(path)
        .status()
        .unwrap();
    assert!(status.success());
}
fn snapshot(root: &Path) -> Vec<(String, Vec<u8>)> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                if path.file_name().is_some_and(|n| n == ".git") {
                    continue;
                }
                walk(root, &path, out);
            } else {
                let name = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                out.push((name, fs::read(&path).unwrap()));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

#[test]
fn unusable_project_paths_name_the_path_for_every_command() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("notes.md"), "# Notes\n").unwrap();
    for command in ["dev", "build", "graph", "check", "init", "setup", "context"] {
        let missing = entwine(temp.path(), &[command, "no-such-dir"]);
        let stderr = text(&missing.stderr);
        assert!(!missing.status.success(), "{command}");
        assert!(
            stderr.contains("Project directory no-such-dir does not exist"),
            "{command}: {stderr}"
        );
        assert!(!stderr.contains("os error"), "{command}: {stderr}");

        let file = entwine(temp.path(), &[command, "notes.md"]);
        let stderr = text(&file.stderr);
        assert!(!file.status.success(), "{command}");
        assert!(
            stderr.contains("Project path notes.md is not a directory"),
            "{command}: {stderr}"
        );
    }
    // Rejected paths never create files.
    assert_eq!(snapshot(temp.path()).len(), 1); // only notes.md
}

#[test]
fn help_describes_every_argument_and_option() {
    let temp = tempfile::tempdir().unwrap();
    for (command, needle) in [
        ("check", "Project directory containing docs/"),
        ("build", "Project directory containing docs/"),
        ("init", "Project directory containing docs/"),
        ("setup", "Project directory containing docs/"),
        ("context", "Emit versioned structured JSON"),
        ("dev", "Loopback port to serve on"),
    ] {
        let help = text(&entwine(temp.path(), &[command, "--help"]).stdout);
        assert!(help.contains(needle), "{command}: {help}");
    }
}

#[test]
fn build_reminds_to_ignore_dist_until_git_ignores_it() {
    let temp = tempfile::tempdir().unwrap();
    git_init(temp.path());
    let init = entwine(temp.path(), &["init", "--yes"]);
    assert!(init.status.success(), "{}", text(&init.stderr));

    let first = entwine(temp.path(), &["build"]);
    assert!(first.status.success(), "{}", text(&first.stderr));
    assert!(text(&first.stderr).contains("Tip: dist/ is not ignored by Git"));

    fs::write(temp.path().join(".gitignore"), "dist/\n").unwrap();
    let second = entwine(temp.path(), &["build"]);
    assert!(second.status.success());
    assert!(!text(&second.stderr).contains("Tip:"));
}

#[test]
fn build_outside_a_git_repository_stays_quiet_about_gitignore() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join("docs")).unwrap();
    fs::write(temp.path().join("docs/index.md"), "# Home\n").unwrap();
    let result = entwine(temp.path(), &["build"]);
    assert!(result.status.success(), "{}", text(&result.stderr));
    assert!(!text(&result.stderr).contains("Tip:"));
}

#[test]
fn first_use_journey_is_non_destructive_and_deterministic() {
    let temp = tempfile::tempdir().unwrap();
    git_init(temp.path());
    fs::write(temp.path().join("README.md"), "# Widget\n").unwrap();
    Command::new("git")
        .args(["remote", "add", "origin", "git@github.com:acme/widget.git"])
        .current_dir(temp.path())
        .status()
        .unwrap();

    // A dry run and a refused non-interactive init change nothing.
    let before = snapshot(temp.path());
    assert!(entwine(temp.path(), &["init", "--dry-run"])
        .status
        .success());
    let refused = entwine(temp.path(), &["init"]);
    assert!(text(&refused.stdout).contains("Re-run with --yes"));
    assert_eq!(snapshot(temp.path()), before);

    // docs/ is missing, so validation fails clearly before any init.
    let early = entwine(temp.path(), &["check"]);
    assert!(!early.status.success());
    assert!(text(&early.stderr).contains("No docs/ directory"));

    assert!(entwine(temp.path(), &["init", "--yes"]).status.success());
    let check = entwine(temp.path(), &["check"]);
    assert!(check.status.success(), "{}", text(&check.stderr));

    // Scaffold has no decisions or specs yet: strict mode fails, plain mode passes.
    let strict = entwine(temp.path(), &["check", "--strict-knowledge"]);
    assert!(!strict.status.success());
    assert!(text(&strict.stderr).contains("2 recommended knowledge areas are missing"));

    // A second init preserves edits.
    fs::write(temp.path().join("docs/state.md"), "# State\n\nMine.\n").unwrap();
    assert!(entwine(temp.path(), &["init", "--yes"]).status.success());
    assert_eq!(
        fs::read_to_string(temp.path().join("docs/state.md")).unwrap(),
        "# State\n\nMine.\n"
    );

    // setup --dry-run writes nothing; setup writes only the workflow and is idempotent.
    let before = snapshot(temp.path());
    let plan = entwine(temp.path(), &["setup", "--dry-run"]);
    assert!(text(&plan.stdout).contains(".github/workflows/entwine.yml"));
    assert_eq!(snapshot(temp.path()), before);
    assert!(entwine(temp.path(), &["setup"]).status.success());
    assert!(temp.path().join(".github/workflows/entwine.yml").is_file());
    let again = text(&entwine(temp.path(), &["setup"]).stdout);
    assert!(again.contains("already configured"));

    // Build output and context are deterministic.
    assert!(entwine(temp.path(), &["build"]).status.success());
    let graph = temp.path().join("dist/__entwine/graph/index.html");
    let first = fs::read(&graph).unwrap();
    assert!(entwine(temp.path(), &["build"]).status.success());
    assert_eq!(fs::read(&graph).unwrap(), first);
    let a = entwine(temp.path(), &["context", "--json"]).stdout;
    let b = entwine(temp.path(), &["context", "--json"]).stdout;
    assert_eq!(a, b);
    assert!(serde_json::from_slice::<serde_json::Value>(&a).is_ok());
}

#[test]
fn validation_failure_names_the_file_and_leaves_no_output() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join("docs")).unwrap();
    fs::write(
        temp.path().join("docs/index.md"),
        "# Home\n\n[gone](missing.md)\n",
    )
    .unwrap();
    for command in ["check", "build", "context"] {
        let result = entwine(temp.path(), &[command]);
        assert!(!result.status.success(), "{command}");
        assert!(
            text(&result.stderr).contains("docs/index.md:3"),
            "{command}"
        );
    }
    assert!(!temp.path().join("dist").exists());
}

#[test]
fn invalid_config_reports_the_offending_line() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join("docs")).unwrap();
    fs::write(temp.path().join("docs/index.md"), "# Home\n").unwrap();
    fs::write(
        temp.path().join("entwine.toml"),
        "[site]\ninclude_agent_knowledge = true\n\n[site]\nrenderer = \"builtin\"\n",
    )
    .unwrap();
    let result = entwine(temp.path(), &["check"]);
    assert!(!result.status.success());
    let stderr = text(&result.stderr);
    assert!(stderr.contains("Invalid entwine.toml:4:"), "{stderr}");

    fs::write(
        temp.path().join("entwine.toml"),
        "[site]\nrenderr = \"x\"\n",
    )
    .unwrap();
    let typo = text(&entwine(temp.path(), &["check"]).stderr);
    assert!(typo.contains("Invalid entwine.toml:2:"), "{typo}");
}
