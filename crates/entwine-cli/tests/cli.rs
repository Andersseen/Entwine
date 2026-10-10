use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

fn fixture() -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/kitchen-sink");
    fn copy(source: &Path, destination: &Path) {
        fs::create_dir_all(destination).unwrap();
        for entry in fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            if entry.file_name() == "dist" {
                continue;
            }
            let target = destination.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy(&entry.path(), &target);
            } else {
                fs::copy(entry.path(), target).unwrap();
            }
        }
    }
    copy(&source, temp.path());
    temp
}
fn cli(command: &str, project: &Path, flags: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_entwine"))
        .arg(command)
        .arg(project)
        .args(flags)
        .output()
        .unwrap()
}

#[test]
fn real_cli_build_check_graph_and_context_work() {
    let temp = fixture();
    for command in ["check", "build", "graph"] {
        let result = cli(command, temp.path(), &[]);
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
    for file in [
        "index.html",
        "architecture/index.html",
        "specs/authentication/index.html",
        "__entwine/graph/index.html",
    ] {
        assert!(temp.path().join("dist").join(file).is_file());
    }
    let context = cli("context", temp.path(), &["--json"]);
    assert!(context.status.success());
    let parsed: serde_json::Value = serde_json::from_slice(&context.stdout).unwrap();
    // 12 documents plus 4 instruction files and 2 skills discovered through entwine.toml.
    assert_eq!(parsed["documents"].as_array().unwrap().len(), 18);
    assert_eq!(parsed["schema_version"], "0.4");
    assert_eq!(
        context.stdout,
        cli("context", temp.path(), &["--json"]).stdout
    );
    let markdown = cli("context", temp.path(), &[]);
    assert!(markdown.status.success());
    assert!(String::from_utf8(markdown.stdout)
        .unwrap()
        .contains("# Project: Harbor"));
    let current_dir = Command::new(env!("CARGO_BIN_EXE_entwine"))
        .arg("check")
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(current_dir.status.success());
}

#[test]
#[cfg(feature = "flowview-renderer")]
fn flowview_is_opt_in_and_keeps_entwine_generated_views() {
    let temp = fixture();
    let config = temp.path().join("entwine.toml");
    let original = fs::read_to_string(&config).unwrap();
    fs::write(&config, format!("{original}\nrenderer = \"flowview\"\n")).unwrap();

    let result = cli("build", temp.path(), &[]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    for file in [
        "index.html",
        "architecture/index.html",
        "specs/authentication/index.html",
        "__entwine/graph/index.html",
        "__entwine/knowledge/index.html",
        "__entwine/agents/index.html",
    ] {
        assert!(temp.path().join("dist").join(file).is_file(), "{file}");
    }
    let html =
        fs::read_to_string(temp.path().join("dist/specs/authentication/index.html")).unwrap();
    for fragment in [
        "<main id=\"main\" tabindex=\"-1\">",
        "aria-label=\"Mobile documentation\"",
        "aria-current=\"page\"",
        "References",
        "Referenced by",
        "Source:",
        "?focus=specs%2Fauthentication.md",
    ] {
        assert!(html.contains(fragment), "missing {fragment}");
    }
}

#[test]
#[cfg(not(feature = "flowview-renderer"))]
fn explicitly_selecting_flowview_without_feature_returns_actionable_error() {
    let temp = fixture();
    let config = temp.path().join("entwine.toml");
    let original = fs::read_to_string(&config).unwrap();
    fs::write(&config, format!("{original}\nrenderer = \"flowview\"\n")).unwrap();

    let result = cli("build", temp.path(), &[]);
    assert!(!result.status.success());
    let error = String::from_utf8_lossy(&result.stderr);
    assert!(error.contains("experimental and was not included"), "{error}");
    assert!(error.contains("--features flowview-renderer"), "{error}");
}

#[test]
fn failing_validation_preserves_last_output_and_sets_exit_code() {
    let temp = fixture();
    assert!(cli("build", temp.path(), &[]).status.success());
    let home = temp.path().join("dist/index.html");
    let original = fs::read(&home).unwrap();
    fs::write(
        temp.path().join("docs/index.md"),
        "# Broken\n[Missing](security.md)",
    )
    .unwrap();
    for command in ["check", "build", "graph", "context"] {
        let result = cli(command, temp.path(), &[]);
        assert!(!result.status.success());
        let stderr = String::from_utf8(result.stderr).unwrap();
        assert!(stderr.contains("docs/index.md:2"));
        assert!(stderr.contains("security.md"));
    }
    assert_eq!(fs::read(home).unwrap(), original);
}

#[test]
fn rebuild_removes_stale_routes_and_refuses_unrelated_dist() {
    let temp = fixture();
    fs::create_dir(temp.path().join("dist")).unwrap();
    fs::write(temp.path().join("dist/precious.txt"), "keep").unwrap();
    assert!(!cli("build", temp.path(), &[]).status.success());
    assert_eq!(
        fs::read_to_string(temp.path().join("dist/precious.txt")).unwrap(),
        "keep"
    );
    fs::remove_dir_all(temp.path().join("dist")).unwrap();
    fs::write(temp.path().join("docs/temporary.md"), "# Temporary").unwrap();
    assert!(cli("build", temp.path(), &[]).status.success());
    assert!(temp.path().join("dist/temporary/index.html").is_file());
    fs::remove_file(temp.path().join("docs/temporary.md")).unwrap();
    assert!(cli("build", temp.path(), &[]).status.success());
    assert!(!temp.path().join("dist/temporary").exists());
}

#[test]
fn dedicated_failures_exit_nonzero() {
    for fixture in ["broken-link", "route-collision", "invalid-frontmatter"] {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../entwine-engine/tests/fixtures")
            .join(fixture);
        assert!(!cli("check", &root, &[]).status.success());
    }
}

struct DevProcess(std::process::Child);
impl Drop for DevProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn get(port: u16, route: &str) -> Option<String> {
    use std::io::{Read, Write};
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).ok()?;
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(2)))
        .ok()?;
    stream
        .write_all(
            format!("GET {route} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
        .ok()?;
    let mut response = String::new();
    stream.read_to_string(&mut response).ok()?;
    Some(response)
}

#[test]
fn dev_serves_rebuilds_preserves_last_good_output_and_rejects_traversal() {
    use std::time::{Duration, Instant};
    let temp = fixture();
    let stderr_log = temp.path().join("dev-stderr.log");
    // A free port picked up front can be taken before `dev` binds it on a busy
    // runner, so retry on a fresh port when the server exits during startup.
    let mut attempt = 0;
    let (mut process, port) = loop {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let mut process = DevProcess(
            Command::new(env!("CARGO_BIN_EXE_entwine"))
                .arg("dev")
                .arg(temp.path())
                .args(["--port", &port.to_string()])
                .stdout(std::process::Stdio::null())
                .stderr(fs::File::create(&stderr_log).unwrap())
                .spawn()
                .unwrap(),
        );
        let started = Instant::now();
        while Instant::now() < started + Duration::from_secs(10) {
            if get(port, "/").is_some_and(|r| r.contains(">Harbor</h1>")) {
                break;
            }
            if process.0.try_wait().unwrap().is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        if process.0.try_wait().unwrap().is_none() {
            break (process, port);
        }
        attempt += 1;
        assert!(
            attempt < 3,
            "dev exited during startup: {}",
            fs::read_to_string(&stderr_log).unwrap_or_default()
        );
    };
    fn wait_for(port: u16, text: &str, process: &mut DevProcess) -> String {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(response) = get(port, "/") {
                if response.contains(text) {
                    return response;
                }
            }
            assert!(
                process.0.try_wait().unwrap().is_none(),
                "dev exited before serving {text}"
            );
            assert!(Instant::now() < deadline, "dev did not serve {text}");
            std::thread::sleep(Duration::from_millis(25));
        }
    }
    let response = wait_for(port, ">Harbor</h1>", &mut process);
    assert!(response.starts_with("HTTP/1.1 200"));
    assert!(get(port, "/architecture/")
        .unwrap()
        .contains("Harbor architecture"));
    assert!(get(port, "/architecture")
        .unwrap()
        .starts_with("HTTP/1.1 308"));
    assert!(get(port, "/%2e%2e/Cargo.toml")
        .unwrap()
        .starts_with("HTTP/1.1 403"));
    fs::write(
        temp.path().join("docs/index.md"),
        "# Repository watch\n[Repository](../README.md)",
    )
    .unwrap();
    wait_for(port, ">Repository watch</h1>", &mut process);
    let published = temp.path().join("dist/index.html");
    let previous = fs::metadata(&published).unwrap().modified().unwrap();
    fs::remove_file(temp.path().join("README.md")).unwrap();
    std::thread::sleep(Duration::from_millis(600));
    assert_eq!(
        fs::metadata(&published).unwrap().modified().unwrap(),
        previous
    );
    assert!(get(port, "/").unwrap().contains(">Repository watch</h1>"));
    fs::write(temp.path().join("README.md"), "# Restored repository file").unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while fs::metadata(&published)
        .ok()
        .and_then(|m| m.modified().ok())
        .is_none_or(|modified| modified == previous)
    {
        assert!(
            Instant::now() < deadline,
            "repository reference change did not rebuild"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
    fs::write(
        temp.path().join("docs/index.md"),
        "# Updated project\n\n[Architecture](architecture.md)",
    )
    .unwrap();
    wait_for(port, ">Updated project</h1>", &mut process);
    fs::write(
        temp.path().join("docs/index.md"),
        "# Broken update\n\n[Bad](missing.md)",
    )
    .unwrap();
    std::thread::sleep(Duration::from_millis(600));
    assert!(get(port, "/").unwrap().contains(">Updated project</h1>"));
    fs::write(
        temp.path().join("docs/index.md"),
        "# Missing repository target\n[New](../source/new.txt)",
    )
    .unwrap();
    std::thread::sleep(Duration::from_millis(600));
    assert!(get(port, "/").unwrap().contains(">Updated project</h1>"));
    fs::create_dir(temp.path().join("source")).unwrap();
    std::thread::sleep(Duration::from_millis(400));
    fs::write(
        temp.path().join("source/new.txt"),
        "created after invalid edit",
    )
    .unwrap();
    wait_for(port, ">Missing repository target</h1>", &mut process);
    fs::write(temp.path().join("docs/index.md"), "# Recovered project").unwrap();
    wait_for(port, ">Recovered project</h1>", &mut process);
    fs::create_dir_all(temp.path().join("docs/new/deep")).unwrap();
    fs::write(
        temp.path().join("docs/new/deep/page.md"),
        "# New nested page",
    )
    .unwrap();
    fs::write(temp.path().join("docs/new/image.svg"), "<svg/>").unwrap();
    fs::write(
        temp.path().join("docs/index.md"),
        "# Nested project\n[Directory](new/)\n![Asset](new/image.svg)",
    )
    .unwrap();
    wait_for(port, ">Nested project</h1>", &mut process);
    assert!(get(port, "/new/").unwrap().contains("New nested page"));
    assert!(get(port, "/new/image.svg")
        .unwrap()
        .starts_with("HTTP/1.1 200"));
    fs::rename(
        temp.path().join("docs/new/deep/page.md"),
        temp.path().join("docs/new/deep/renamed.md"),
    )
    .unwrap();
    fs::write(
        temp.path().join("docs/index.md"),
        "# Renamed project\n[Directory](new/)",
    )
    .unwrap();
    wait_for(port, ">Renamed project</h1>", &mut process);
    assert!(get(port, "/new/deep/page/")
        .unwrap()
        .starts_with("HTTP/1.1 404"));
    assert!(get(port, "/new/deep/renamed/")
        .unwrap()
        .starts_with("HTTP/1.1 200"));
    fs::remove_dir_all(temp.path().join("docs/new")).unwrap();
    fs::write(temp.path().join("docs/index.md"), "# Deleted project").unwrap();
    wait_for(port, ">Deleted project</h1>", &mut process);
    assert!(get(port, "/new/").unwrap().starts_with("HTTP/1.1 404"));
    assert!(get(port, "/new/image.svg")
        .unwrap()
        .starts_with("HTTP/1.1 404"));
}

#[test]
fn check_reports_agent_knowledge_only_when_configured_and_rejects_bad_config() {
    let temp = fixture();
    let configured = cli("check", temp.path(), &[]);
    let report = String::from_utf8_lossy(&configured.stderr).into_owned();
    assert!(configured.status.success(), "{report}");
    assert!(
        report.contains("4 agent instruction files in 3 scopes"),
        "{report}"
    );
    assert!(report.contains("2 skills"), "{report}");
    assert!(
        report.contains("published in the generated site"),
        "{report}"
    );
    // Publication is its own switch: discovery stays, the public site loses the files.
    fs::write(
        temp.path().join("entwine.toml"),
        "[discovery]\nagent_instructions = true\nskills = true\n",
    )
    .unwrap();
    assert!(cli("build", temp.path(), &[]).status.success());
    assert!(!temp.path().join("dist/__entwine/agents").exists());
    let context = cli("context", temp.path(), &["--json"]);
    let parsed: serde_json::Value = serde_json::from_slice(&context.stdout).unwrap();
    assert_eq!(parsed["documents"].as_array().unwrap().len(), 18);
    let report = String::from_utf8_lossy(&cli("check", temp.path(), &[]).stderr).into_owned();
    assert!(report.contains("not published; context only"), "{report}");
    // An empty config equals no config: zero-config behavior, agent files ignored, nothing reported.
    fs::write(temp.path().join("entwine.toml"), "").unwrap();
    let report = String::from_utf8_lossy(&cli("check", temp.path(), &[]).stderr).into_owned();
    assert!(!report.contains("agent instruction"), "{report}");
    let context = cli("context", temp.path(), &["--json"]);
    let parsed: serde_json::Value = serde_json::from_slice(&context.stdout).unwrap();
    assert_eq!(parsed["documents"].as_array().unwrap().len(), 12);
    // A typo is an error, never a silent default.
    fs::write(
        temp.path().join("entwine.toml"),
        "[site]\ninclude_agent_knowlege = true\n",
    )
    .unwrap();
    let broken = cli("check", temp.path(), &[]);
    assert!(!broken.status.success());
    assert!(String::from_utf8_lossy(&broken.stderr).contains("Invalid entwine.toml"));
}
