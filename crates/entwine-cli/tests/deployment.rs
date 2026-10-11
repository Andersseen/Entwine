//! Deployment contracts, verified locally and deterministically.
//!
//! Nothing here contacts GitHub, GitLab or Bitbucket. Three kinds of evidence:
//! 1. a static-host simulator crawls real `entwine build` output mounted at the root and
//!    under subpaths, resolving every reference the way a browser would;
//! 2. generated pipelines are parsed as YAML and checked against provider semantics and
//!    against the artifact paths the real CLI produces;
//! 3. the Bitbucket publish script runs for real against a local bare Git repository.
//!
//! Hosted execution is separate evidence; see docs/deployment-validation.md.
use percent_encoding::percent_decode_str;
use serde_yaml::Value;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn entwine(args: &[&str], cwd: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_entwine"))
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap()
}
fn ok(output: Output) -> String {
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}
fn git(cwd: &Path, args: &[&str]) {
    ok(Command::new("git")
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
        .unwrap());
}
fn write(root: &Path, relative: &str, text: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// A project with nested routes, names needing percent-encoding, case differences, local
/// assets, anchors, repository references and published agent knowledge.
fn write_project(project: &Path) {
    write(project, "entwine.toml", "[discovery]\nagent_instructions = true\nskills = true\n\n[site]\ninclude_agent_knowledge = true\n");
    write(project, "AGENTS.md", "# Agents\nUse the docs.\n");
    write(project, "src/my lib.rs", "// code\n");
    write(project, "docs/index.md", "# Home\n\n[Guide](guide/Deep%20Nest/Page%20One.md) [Accent](caf%C3%A9/menu.md) [Plus](a+b/c%26d.md#section-b) [Code](../src/my%20lib.rs)\n\n![Diagram](img/pic%20a.png)\n\n## Section A\n");
    write(project, "docs/guide/Deep Nest/Page One.md", "# Page One\n\n[home](../../index.md#section-a) [sub](sub/leaf.md#deep-heading) [Menu](../../café/menu.md)\n");
    write(
        project,
        "docs/guide/Deep Nest/sub/leaf.md",
        "# Leaf\n\n## Deep heading\n\n[back](../Page%20One.md)\n",
    );
    write(
        project,
        "docs/café/menu.md",
        "# Menu\n\n[home](../index.md)\n",
    );
    write(
        project,
        "docs/a+b/c&d.md",
        "# Section B\n\n[home](../index.md)\n",
    );
    write(project, "docs/img/pic a.png", "not-really-a-png");
    write(
        project,
        "docs/Upper/Case.md",
        "# Upper\n\n[home](../index.md)\n",
    );
}

fn built(layout: &str) -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join(layout);
    write_project(&project);
    ok(entwine(&["check", project.to_str().unwrap()], temp.path()));
    ok(entwine(&["build", project.to_str().unwrap()], temp.path()));
    let dist = project.join("dist");
    (temp, dist)
}

// ---- static host simulator --------------------------------------------------------------

/// Exact-case, byte-exact existence, as on a Linux static host (macOS volumes are not).
fn exact_entry(root: &Path, relative: &[String]) -> Option<PathBuf> {
    let mut current = root.to_path_buf();
    for part in relative {
        let found = fs::read_dir(&current)
            .ok()?
            .flatten()
            .find(|e| e.file_name().to_str() == Some(part.as_str()))?;
        current = found.path();
    }
    Some(current)
}

fn unescape(value: &str) -> String {
    value
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// `href`/`src` attribute values and the `"href"` strings of the embedded graph JSON.
fn references(html: &str) -> Vec<String> {
    let mut found = Vec::new();
    for attribute in [" href=\"", " src=\"", "\"href\":\""] {
        let mut rest = html;
        while let Some(at) = rest.find(attribute) {
            rest = &rest[at + attribute.len()..];
            let end = rest.find('"').unwrap();
            found.push(unescape(&rest[..end]));
            rest = &rest[end..];
        }
    }
    found
}

/// RFC 3986 relative resolution of a path against a base path, dot segments removed.
fn resolve(base: &str, reference: &str) -> String {
    let path = reference.split(['#', '?']).next().unwrap();
    let joined = if path.is_empty() {
        base.to_string()
    } else if path.starts_with('/') {
        path.to_string()
    } else {
        format!("{}{path}", &base[..=base.rfind('/').unwrap()])
    };
    let mut out: Vec<&str> = Vec::new();
    let segments: Vec<&str> = joined.split('/').collect();
    for (i, segment) in segments.iter().enumerate() {
        match *segment {
            "." => {}
            ".." => {
                out.pop();
            }
            "" if i != 0 && i != segments.len() - 1 => {}
            other => out.push(other),
        }
    }
    let mut url = out.join("/");
    if matches!(segments.last(), Some(&".") | Some(&"..")) && !url.ends_with('/') {
        url.push('/');
    }
    url
}

#[derive(Default)]
struct Crawl {
    pages: BTreeSet<String>,
    external: BTreeSet<String>,
    checked: usize,
}

/// Fetch `url` the way a static host would: directories need the trailing slash and are
/// never silently redirected, because relative links computed from the redirect target
/// would differ.
fn fetch(dist: &Path, prefix: &str, url: &str) -> Result<PathBuf, String> {
    let inner = url
        .strip_prefix(prefix.trim_end_matches('/'))
        .filter(|rest| rest.starts_with('/'))
        .ok_or_else(|| format!("{url} escapes the base {prefix}"))?;
    let decoded = percent_decode_str(&inner[1..])
        .decode_utf8()
        .map_err(|_| format!("{url} is not valid UTF-8 after decoding"))?;
    let parts: Vec<String> = decoded
        .split('/')
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect();
    let target = exact_entry(dist, &parts).ok_or_else(|| format!("404 {url}"))?;
    if target.is_dir() {
        if !url.ends_with('/') {
            return Err(format!("{url} is a directory without a trailing slash"));
        }
        let index = target.join("index.html");
        return if index.is_file() {
            Ok(index)
        } else {
            Err(format!("403 {url} has no index.html"))
        };
    }
    if url.ends_with('/') {
        return Err(format!("404 {url} (file addressed as directory)"));
    }
    Ok(target)
}

fn crawl(dist: &Path, prefix: &str) -> Crawl {
    let mut result = Crawl::default();
    let mut queue = VecDeque::from([prefix.to_string()]);
    while let Some(url) = queue.pop_front() {
        if !result.pages.insert(url.clone()) {
            continue;
        }
        let file = fetch(dist, prefix, &url).unwrap_or_else(|e| panic!("{e}"));
        if file.extension().and_then(|e| e.to_str()) != Some("html") {
            continue;
        }
        let html = fs::read_to_string(&file).unwrap();
        for reference in references(&html) {
            if reference.starts_with("https://") {
                result.external.insert(reference);
                continue;
            }
            assert!(
                !reference.starts_with('/') && !reference.contains("://"),
                "{url}: base-dependent reference {reference:?}"
            );
            let target = resolve(&url, &reference);
            let file = fetch(dist, prefix, &target)
                .unwrap_or_else(|e| panic!("{url} -> {reference:?}: {e}"));
            if let Some((_, fragment)) = reference.split_once('#') {
                let fragment = percent_decode_str(fragment).decode_utf8().unwrap();
                let body = fs::read_to_string(&file).unwrap();
                assert!(
                    body.contains(&format!("id=\"{fragment}\"")),
                    "{url} -> {reference:?}: #{fragment} missing from {target}"
                );
            }
            result.checked += 1;
            queue.push_back(target);
        }
    }
    result
}

fn html_routes(dist: &Path) -> BTreeSet<String> {
    fn walk(dir: &Path, base: &str, out: &mut BTreeSet<String>) {
        for entry in fs::read_dir(dir).unwrap().flatten() {
            let name = entry.file_name().into_string().unwrap();
            if entry.path().is_dir() {
                walk(&entry.path(), &format!("{base}{name}/"), out);
            } else if name == "index.html" {
                out.insert(base.to_string());
            }
        }
    }
    let mut out = BTreeSet::new();
    walk(dist, "/", &mut out);
    out
}

#[test]
fn static_output_resolves_at_the_root_and_under_provider_subpaths() {
    let (_temp, dist) = built("project");
    let routes = html_routes(&dist);
    // GitHub project pages, GitLab project pages and nested GitLab groups,
    // Bitbucket `<workspace>.bitbucket.io/<repository>/`, and a root deployment.
    for prefix in ["/", "/widgets/", "/group/sub-group/widgets/", "/a%20b/"] {
        let result = crawl(&dist, prefix);
        let reached: BTreeSet<String> = result
            .pages
            .iter()
            .filter(|url| !url.contains("style.css") && !url.contains("graph.js"))
            .filter(|url| url.ends_with('/'))
            .map(|url| {
                format!(
                    "/{}",
                    percent_decode_str(&url[prefix.len()..])
                        .decode_utf8()
                        .unwrap()
                )
            })
            .collect();
        // Every generated page is reachable from the entry page by links alone.
        assert_eq!(
            reached.difference(&routes).count(),
            0,
            "{prefix}: links reach pages that do not exist"
        );
        let unreachable: Vec<_> = routes.difference(&reached).collect();
        assert!(
            unreachable.is_empty(),
            "{prefix}: pages unreachable from the home page: {unreachable:?}"
        );
        assert!(result.checked > 40, "{prefix}: crawl was too shallow");
        for route in [
            "__entwine/graph/",
            "__entwine/knowledge/",
            "__entwine/agents/",
            "guide/Deep%20Nest/Page%20One/",
            "caf%C3%A9/menu/",
            "a%2Bb/c%26d/",
            "Upper/Case/",
        ] {
            assert!(
                result.pages.contains(&format!("{prefix}{route}")),
                "{prefix}: {route} was never linked"
            );
        }
        // Assets referenced from content, with an encoded space.
        assert!(result.pages.contains(&format!("{prefix}img/pic%20a.png")));
    }
}

#[test]
fn repository_references_are_provider_urls_independent_of_the_pages_base() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path();
    write_project(project);
    git(project, &["init", "-q"]);
    git(
        project,
        &[
            "remote",
            "add",
            "origin",
            "git@gitlab.com:group/sub/widgets.git",
        ],
    );
    git(project, &["add", "-A"]);
    git(project, &["commit", "-q", "-m", "fixture"]);
    ok(entwine(&["build"], project));
    let dist = project.join("dist");
    let result = crawl(&dist, "/widgets/");
    let source: Vec<_> = result
        .external
        .iter()
        .filter(|u| u.contains("my%20lib.rs"))
        .collect();
    assert_eq!(source.len(), 1, "{:?}", result.external);
    assert!(source[0].starts_with("https://gitlab.com/group/sub/widgets/-/blob/"));
    for url in &result.external {
        assert!(
            !url.contains("/widgets/#") && !url.contains("pages"),
            "{url}"
        );
        assert!(!url.contains(' '), "{url} is not encoded");
    }
}

#[test]
fn output_is_deterministic_and_independent_of_the_checkout_location() {
    let (_a, first) = built("one");
    let (_b, second) = built("a/much/deeper/checkout/path");
    let tree = |root: &Path| -> BTreeMap<String, Vec<u8>> {
        fn walk(dir: &Path, root: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
            for entry in fs::read_dir(dir).unwrap().flatten() {
                if entry.path().is_dir() {
                    walk(&entry.path(), root, out);
                } else {
                    let name = entry
                        .path()
                        .strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned();
                    out.insert(name, fs::read(entry.path()).unwrap());
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(root, root, &mut out);
        out.remove(".entwine-output");
        out
    };
    assert_eq!(tree(&first), tree(&second));
}

#[test]
fn an_empty_site_fails_with_an_actionable_error_in_the_validation_step() {
    let temp = tempfile::tempdir().unwrap();
    // No docs/ at all, then an empty docs/: `check` runs before `build` in every pipeline.
    for create_docs in [false, true] {
        if create_docs {
            fs::create_dir(temp.path().join("docs")).unwrap();
        }
        for command in ["check", "build"] {
            let output = entwine(&[command], temp.path());
            assert!(!output.status.success());
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(stderr.contains("docs/index.md"), "{stderr}");
        }
        assert!(!temp.path().join("dist").exists());
    }
}

// ---- provider pipelines -----------------------------------------------------------------

/// A repository with the project at `layout` ("" for the repository root).
fn repository(remote: &str, layout: &str) -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    git(temp.path(), &["init", "-q"]);
    git(temp.path(), &["remote", "add", "origin", remote]);
    let project = temp.path().join(layout);
    write_project(&project);
    (temp, project)
}

fn yaml(root: &Path, relative: &str) -> Value {
    serde_yaml::from_str(&fs::read_to_string(root.join(relative)).unwrap()).unwrap()
}
fn strings(value: &Value) -> Vec<String> {
    value
        .as_sequence()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect()
}

/// The command `entwine build <project>` as written in a pipeline, run from the repository
/// root exactly as a CI checkout would, and the directory it must have produced.
fn run_pipeline_commands(root: &Path, scripts: &[String], project_arg: &str) {
    for line in scripts.iter().filter(|l| l.starts_with("entwine ")) {
        let expected = format!(" {project_arg}");
        let args: Vec<&str> = line.split_whitespace().skip(1).collect();
        assert!(
            args.len() == 1 + usize::from(!project_arg.is_empty()),
            "unexpected arguments in {line:?}"
        );
        assert_eq!(
            args.get(1).map(|a| format!(" {a}")).unwrap_or_default(),
            if project_arg.is_empty() {
                String::new()
            } else {
                expected
            }
        );
        ok(entwine(&args, root));
    }
}

#[test]
fn github_workflow_follows_pages_semantics_and_uploads_the_real_build_output() {
    for layout in ["", "apps/site"] {
        let (temp, project) = repository("git@github.com:acme/widgets.git", layout);
        ok(entwine(&["setup", project.to_str().unwrap()], temp.path()));
        let wf = yaml(temp.path(), ".github/workflows/entwine.yml");

        let jobs = wf["jobs"].as_mapping().unwrap();
        assert_eq!(jobs.len(), 2);
        // Least privilege: read-only by default; Pages and OIDC only on the deploy job.
        assert_eq!(wf["permissions"]["contents"], "read");
        assert!(wf["permissions"].as_mapping().unwrap().len() == 1);
        assert!(wf["jobs"]["build"]["permissions"].is_null());
        assert_eq!(wf["jobs"]["deploy"]["permissions"]["pages"], "write");
        assert_eq!(wf["jobs"]["deploy"]["permissions"]["id-token"], "write");
        assert_eq!(wf["jobs"]["deploy"]["needs"], "build");
        assert_eq!(wf["jobs"]["deploy"]["environment"]["name"], "github-pages");
        assert_eq!(
            wf["jobs"]["deploy"]["environment"]["url"],
            "${{ steps.deployment.outputs.page_url }}"
        );
        assert_eq!(wf["jobs"]["deploy"]["steps"][0]["id"], "deployment");
        assert_eq!(
            wf["jobs"]["deploy"]["concurrency"]["cancel-in-progress"],
            false
        );
        assert!(wf["on"]["pull_request"].is_mapping() && wf["on"]["push"].is_mapping());
        assert!(wf["on"]["workflow_dispatch"].is_null());
        // Pull requests never publish.
        for condition in ["build", "deploy"] {
            let text = wf["jobs"][condition]["if"].as_str().unwrap();
            assert!(text.contains("pull_request"), "{text}");
        }
        let steps = wf["jobs"]["build"]["steps"].as_sequence().unwrap();
        for step in steps
            .iter()
            .chain(wf["jobs"]["deploy"]["steps"].as_sequence().unwrap())
        {
            if let Some(uses) = step["uses"].as_str() {
                assert!(
                    uses.split_once('@')
                        .is_some_and(|(_, v)| v.starts_with('v')),
                    "{uses} is not version-pinned"
                );
            }
            if step["name"]
                .as_str()
                .is_some_and(|n| n.contains("Upload") || n.contains("Build"))
            {
                assert_eq!(step["if"], "github.event_name != 'pull_request'");
            }
        }

        // The step ordering that makes upload see the build: check → build → upload.
        let runs: Vec<String> = steps
            .iter()
            .filter_map(|s| s["run"].as_str().map(str::to_string))
            .collect();
        let project_arg = layout;
        let commands: Vec<String> = runs
            .iter()
            .filter(|r| r.starts_with("entwine "))
            .map(|r| r.replace("\"", ""))
            .collect();
        assert_eq!(commands.len(), 2);
        assert!(
            commands[0].starts_with("entwine check") && commands[1].starts_with("entwine build")
        );
        run_pipeline_commands(temp.path(), &commands, project_arg);
        let upload = steps
            .iter()
            .find(|s| {
                s["uses"]
                    .as_str()
                    .is_some_and(|u| u.starts_with("actions/upload-pages-artifact@"))
            })
            .unwrap();
        let path = upload["with"]["path"].as_str().unwrap();
        assert_eq!(
            path,
            if layout.is_empty() {
                "dist".into()
            } else {
                format!("{layout}/dist")
            }
        );
        // The uploaded directory is a Pages artifact root: index.html at its top, no links
        // (Pages rejects symlinks and hard links in the artifact).
        let artifact = temp.path().join(path);
        assert!(artifact.join("index.html").is_file());
        assert_no_links(&artifact);
        let result = crawl(&artifact, "/widgets/");
        assert!(result.checked > 40);
    }
}

fn assert_no_links(dir: &Path) {
    for entry in fs::read_dir(dir).unwrap().flatten() {
        let meta = fs::symlink_metadata(entry.path()).unwrap();
        assert!(!meta.file_type().is_symlink(), "{:?}", entry.path());
        if meta.is_dir() {
            assert_no_links(&entry.path());
        }
    }
}

#[test]
fn gitlab_pipeline_is_valid_pages_ci_and_publishes_the_artifact_it_built() {
    for layout in ["", "docs-site"] {
        let (temp, project) = repository("https://gitlab.com/group/widgets.git", layout);
        ok(entwine(&["setup", project.to_str().unwrap()], temp.path()));
        // The root file includes the generated file, which exists at the included path.
        let root = yaml(temp.path(), ".gitlab-ci.yml");
        let included = root["include"][0]["local"].as_str().unwrap();
        assert!(temp.path().join(included).is_file());
        let ci = yaml(temp.path(), included);
        let map = ci.as_mapping().unwrap();

        // Only hidden templates and real jobs; resolve `extends` like GitLab (one level).
        let job = |name: &str| -> Value {
            let mut merged = ci[".entwine-base"].clone();
            for (k, v) in ci[name].as_mapping().unwrap() {
                merged[k.as_str().unwrap()] = v.clone();
            }
            merged
        };
        let jobs: Vec<&str> = map
            .keys()
            .map(|k| k.as_str().unwrap())
            .filter(|k| !k.starts_with('.'))
            .collect();
        assert_eq!(jobs, ["entwine:check", "entwine:build", "entwine:pages"]);
        let stages = ["build", "test", "deploy"];
        for name in &jobs {
            let j = job(name);
            assert_eq!(ci[name]["extends"], ".entwine-base");
            assert!(stages.contains(&j["stage"].as_str().unwrap()));
            assert!(!strings(&j["script"]).is_empty(), "{name} needs a script");
            assert_eq!(j["image"], "$ENTWINE_IMAGE");
            assert!(j["variables"]["ENTWINE_IMAGE"].as_str().is_some());
            // Every job is rules-gated; none runs for tag, schedule, or fork surprises.
            assert!(
                j["rules"].as_sequence().is_some_and(|r| !r.is_empty()),
                "{name}"
            );
            for rule in j["rules"].as_sequence().unwrap() {
                assert!(rule["if"].as_str().is_some());
                assert!(!rule["changes"].as_sequence().unwrap().is_empty());
            }
        }
        // Merge requests validate only.
        assert!(job("entwine:check")["rules"][0]["if"]
            .as_str()
            .unwrap()
            .contains("merge_request_event"));
        assert!(job("entwine:check")["artifacts"].is_null());
        // Publishing happens only for the default branch, and needs the build job.
        for name in ["entwine:build", "entwine:pages"] {
            assert_eq!(
                job(name)["rules"][0]["if"],
                "$CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH"
            );
        }
        let needs = strings(&job("entwine:pages")["needs"]);
        assert_eq!(needs, ["entwine:build"]);
        assert!(jobs.contains(&needs[0].as_str()));
        assert_eq!(job("entwine:pages")["stage"], "deploy");
        assert_eq!(job("entwine:build")["stage"], "test");

        // `pages.publish` is exactly the directory the build job exposes as an artifact.
        let publish = job("entwine:pages")["pages"]["publish"]
            .as_str()
            .unwrap()
            .to_string();
        let artifact_paths = strings(&job("entwine:build")["artifacts"]["paths"]);
        assert_eq!(artifact_paths, std::slice::from_ref(&publish));
        assert_eq!(
            publish,
            if layout.is_empty() {
                "dist".into()
            } else {
                format!("{layout}/dist")
            }
        );
        assert!(job("entwine:build")["artifacts"]["expire_in"]
            .as_str()
            .is_some());

        let scripts = strings(&job("entwine:build")["script"]);
        let commands: Vec<String> = scripts
            .iter()
            .filter(|s| s.starts_with("entwine "))
            .cloned()
            .collect();
        assert_eq!(commands.len(), 2);
        assert!(scripts
            .iter()
            .any(|s| s.starts_with("npm install --global")));
        run_pipeline_commands(temp.path(), &commands, layout);
        let site = temp.path().join(&publish);
        assert!(site.join("index.html").is_file());
        assert_no_links(&site);
        // GitLab Pages serves at https://group.gitlab.io/widgets/ (or a unique domain at /).
        for prefix in ["/widgets/", "/"] {
            assert!(crawl(&site, prefix).checked > 40);
        }
    }
}

#[test]
fn gitlab_root_file_is_extended_only_when_safe() {
    let (temp, project) = repository("https://gitlab.com/group/widgets.git", "");
    fs::write(temp.path().join(".gitlab-ci.yml"), "stages:\n  - one\n").unwrap();
    let out = ok(entwine(&["setup", project.to_str().unwrap()], temp.path()));
    assert!(out.contains("include:"), "{out}");
    assert_eq!(
        fs::read_to_string(temp.path().join(".gitlab-ci.yml")).unwrap(),
        "stages:\n  - one\n"
    );
}

fn bitbucket_steps(pipeline: &Value) -> Vec<&Value> {
    let mut steps = Vec::new();
    for group in ["pull-requests", "branches"] {
        for (_, list) in pipeline["pipelines"][group].as_mapping().unwrap() {
            for item in list.as_sequence().unwrap() {
                steps.push(&item["step"]);
            }
        }
    }
    steps
}

#[test]
fn bitbucket_pipeline_matches_the_pipelines_schema_and_only_publishes_the_default_branch() {
    for layout in ["", "docs-site"] {
        let (temp, project) = repository("git@bitbucket.org:ws/widgets.git", layout);
        git(temp.path(), &["checkout", "-q", "-b", "trunk"]);
        ok(entwine(&["setup", project.to_str().unwrap()], temp.path()));
        let pipeline = yaml(temp.path(), "bitbucket-pipelines.yml");
        let root = pipeline.as_mapping().unwrap();
        assert_eq!(root.len(), 1, "only `pipelines` at the top level");
        let allowed_steps = ["name", "image", "script", "condition"];
        for step in bitbucket_steps(&pipeline) {
            for key in step.as_mapping().unwrap().keys() {
                assert!(allowed_steps.contains(&key.as_str().unwrap()), "{key:?}");
            }
            assert!(step["name"].as_str().is_some());
            assert_eq!(step["image"], "node:22");
            assert!(!strings(&step["script"]).is_empty());
            let include = strings(&step["condition"]["changesets"]["includePaths"]);
            assert!(include.iter().all(|p| p.ends_with("**")));
        }
        // Pull requests validate; the default branch builds and publishes. No other trigger.
        assert_eq!(pipeline["pipelines"].as_mapping().unwrap().len(), 2);
        let pr = strings(&pipeline["pipelines"]["pull-requests"]["**"][0]["step"]["script"]);
        assert!(pr
            .iter()
            .all(|line| !line.contains("git push") && !line.contains("TOKEN")));
        let branches = pipeline["pipelines"]["branches"].as_mapping().unwrap();
        assert_eq!(branches.len(), 1);
        let (branch, _) = branches.iter().next().unwrap();
        assert!(branch.as_str().unwrap() == "trunk" || branch.as_str().unwrap() == "main");
        let script = strings(
            &pipeline["pipelines"]["branches"][branch.as_str().unwrap()][0]["step"]["script"],
        );
        assert!(script[0].starts_with("npm install --global \"@entwine/cli@"));
        let commands: Vec<String> = script
            .iter()
            .filter(|s| s.starts_with("entwine "))
            .cloned()
            .collect();
        assert_eq!(commands.len(), 2);
        run_pipeline_commands(temp.path(), &commands, layout);
        let secret_hints = script.join("\n");
        assert!(secret_hints.contains("${ENTWINE_SITE_TOKEN:-}"));
        assert!(!secret_hints.contains("https://x-token-auth"));
        assert!(secret_hints.contains(&format!(
            "DIST=\"$BITBUCKET_CLONE_DIR/{}dist\"",
            if layout.is_empty() {
                "".into()
            } else {
                format!("{layout}/")
            }
        )));
    }
}

#[cfg(unix)]
#[test]
fn bitbucket_publish_script_publishes_a_subpath_site_to_the_workspace_repository() {
    // Regression: Bitbucket slugs may contain periods; the script used to reject them.
    const SLUG: &str = "my.docs";
    let (temp, project) = repository("git@bitbucket.org:ws/widgets.git", "");
    git(temp.path(), &["checkout", "-q", "-b", "main"]);
    ok(entwine(&["setup", project.to_str().unwrap()], temp.path()));
    ok(entwine(&["build"], temp.path()));
    let pipeline = yaml(temp.path(), "bitbucket-pipelines.yml");
    let script = pipeline["pipelines"]["branches"]["main"][0]["step"]["script"]
        .as_sequence()
        .unwrap()
        .last()
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();

    // A local bare repository stands in for bitbucket.org/ws/ws.bitbucket.io, with one
    // existing page that other repositories published into the same workspace site.
    let remote = tempfile::tempdir().unwrap();
    let bare = remote.path().join("ws.bitbucket.io.git");
    git(
        remote.path(),
        &["init", "-q", "--bare", bare.to_str().unwrap()],
    );
    let seed = tempfile::tempdir().unwrap();
    git(seed.path(), &["init", "-q"]);
    write(seed.path(), "other-repo/index.html", "<p>other</p>");
    git(seed.path(), &["add", "-A"]);
    git(seed.path(), &["commit", "-q", "-m", "seed"]);
    git(
        seed.path(),
        &["push", "-q", bare.to_str().unwrap(), "HEAD:refs/heads/main"],
    );
    git(&bare, &["symbolic-ref", "HEAD", "refs/heads/main"]);

    let run = |home: &Path| {
        Command::new("sh")
            .args(["-c", &script])
            .current_dir(temp.path())
            .env("HOME", home)
            .env("ENTWINE_SITE_TOKEN", "test-secret-never-printed")
            .env("BITBUCKET_WORKSPACE", "ws")
            .env("BITBUCKET_REPO_SLUG", SLUG)
            .env("BITBUCKET_COMMIT", "abc123")
            .env("BITBUCKET_CLONE_DIR", temp.path())
            .env("GIT_CONFIG_COUNT", "1")
            .env(
                "GIT_CONFIG_KEY_0",
                format!("url.{}/.insteadOf", remote.path().display()),
            )
            .env("GIT_CONFIG_VALUE_0", "https://bitbucket.org/ws/")
            .output()
            .unwrap()
    };
    let home = tempfile::tempdir().unwrap();
    let first = run(home.path());
    let log = format!(
        "{}{}",
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(first.status.success(), "{log}");
    assert!(!log.contains("test-secret"));
    assert!(
        !home.path().join("entwine-askpass.sh").exists(),
        "askpass helper must be removed"
    );

    let published = tempfile::tempdir().unwrap();
    git(
        published.path(),
        &["clone", "-q", bare.to_str().unwrap(), "site"],
    );
    let site = published.path().join("site");
    assert!(
        site.join("other-repo/index.html").is_file(),
        "other repositories' pages survive"
    );
    assert!(!site.join("my.docs/.entwine-output").exists());
    assert!(site.join("my.docs/index.html").is_file());
    // https://ws.bitbucket.io/widgets/ — the documented URL — is a working subpath site.
    let result = crawl(&site.join(SLUG), "/widgets/");
    assert!(result.checked > 40);
    assert!(!site.join("my.docs/widgets").exists());

    // Re-running with unchanged output publishes nothing.
    // Every Bitbucket step starts in a fresh container.
    let fresh = tempfile::tempdir().unwrap();
    let again = run(fresh.path());
    assert!(String::from_utf8_lossy(&again.stdout).contains("No documentation changes to publish."));
}

#[test]
fn unsafe_names_fail_validation_with_actionable_errors_instead_of_broken_urls() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        "docs/index.md",
        "# Home\n[q](what%3F.md) [c](upper/case.md)\n",
    );
    write(temp.path(), "docs/what?.md", "# Q\n");
    write(temp.path(), "docs/Upper/Case.md", "# U\n");
    let output = entwine(&["build"], temp.path());
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    // A name that cannot be a URL path segment is refused; a link whose case differs from
    // the file would 404 on a case-sensitive host, so it is a broken link everywhere.
    assert!(
        stderr.contains("Invalid documentation path: what?.md"),
        "{stderr}"
    );
    assert!(
        stderr.contains("Broken internal link \"upper/case.md\""),
        "{stderr}"
    );
    assert!(stderr.contains("output was not changed"));
    assert!(!temp.path().join("dist").exists());
}

#[test]
fn the_host_simulator_detects_the_defects_it_exists_to_catch() {
    let broken = |files: &[(&str, &str)]| {
        let dir = tempfile::tempdir().unwrap();
        for (name, text) in files {
            write(dir.path(), name, text);
        }
        let path = dir.path().to_path_buf();
        let result = std::panic::catch_unwind(|| crawl(&path, "/repo/"));
        drop(dir);
        result.is_err()
    };
    let page = |body: &str| format!("<a id=\"here\"></a>{body}");
    assert!(!broken(&[
        ("index.html", &page("<a href=\"b/\">b</a>")),
        ("b/index.html", "ok")
    ]));
    assert!(broken(&[
        ("index.html", &page("<a href=\"/b/\">root-absolute</a>")),
        ("b/index.html", "ok")
    ]));
    assert!(broken(&[
        ("index.html", &page("<a href=\"../b/\">escapes</a>")),
        ("b/index.html", "ok")
    ]));
    assert!(broken(&[
        ("index.html", &page("<a href=\"b\">no slash</a>")),
        ("b/index.html", "ok")
    ]));
    assert!(broken(&[
        ("index.html", &page("<a href=\"B/\">case</a>")),
        ("b/index.html", "ok")
    ]));
    assert!(broken(&[(
        "index.html",
        &page("<a href=\"#gone\">frag</a>")
    )]));
    assert!(broken(&[
        ("index.html", &page("<img src=\"a%20b.png\">")),
        ("a b.PNG", "x")
    ]));
}
