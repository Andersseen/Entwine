//! Structural release and cross-platform workflow contracts; hosted execution is separate evidence.
use serde_yaml::Value;
#[test]
fn release_retry_and_visibility_preserve_quality_and_registry_gates() {
    let text = include_str!("../../../.github/workflows/release.yml");
    let workflow: Value = serde_yaml::from_str(text).unwrap();
    assert!(
        workflow["on"]["workflow_dispatch"]["inputs"]["tag"]["required"]
            .as_bool()
            .unwrap()
    );
    assert_eq!(
        workflow["jobs"]["quality"]["with"]["release-ref"]
            .as_str()
            .unwrap(),
        "${{ needs.release-please.outputs.sha }}"
    );
    let needs = workflow["jobs"]["publish-release"]["needs"]
        .as_sequence()
        .unwrap();
    assert!(needs.iter().any(|v| v.as_str() == Some("npm")));
    assert!(needs.iter().any(|v| v.as_str() == Some("github-assets")));
    assert!(text.contains("Retry only an existing draft release"));
    assert!(text.contains(
        "gh workflow run ci.yml --repo \"$GITHUB_REPOSITORY\" --ref \"$RELEASE_BRANCH\""
    ));
    assert_eq!(
        workflow["jobs"]["release-please"]["permissions"]["actions"],
        "write"
    );
    assert!(text.contains("node tooling/check-versions.ts \"$VERSION\""));
    assert!(text.contains("node tooling/smoke-public.ts --version \"$VERSION\""));
    let ci: Value =
        serde_yaml::from_str(include_str!("../../../.github/workflows/ci.yml")).unwrap();
    let matrix = ci["jobs"]["portability"]["strategy"]["matrix"]["os"]
        .as_sequence()
        .unwrap();
    assert!(matrix.iter().any(|v| v.as_str() == Some("macos-14")));
    assert!(matrix.iter().any(|v| v.as_str() == Some("windows-latest")));
}

#[cfg(unix)]
#[test]
fn bitbucket_rejects_hostile_provider_variables_before_any_external_operation() {
    let yaml: Value = serde_yaml::from_str(include_str!("golden/bitbucket.yml")).unwrap();
    let script = yaml["pipelines"]["branches"]["main"][0]["step"]["script"]
        .as_sequence()
        .unwrap()
        .last()
        .unwrap()
        .as_str()
        .unwrap();
    for bad in [
        "../escape",
        "-flag",
        "bad/name",
        "$(touch injected)",
        "space name",
        "",
        "a\nb",
    ] {
        let temporary = tempfile::tempdir().unwrap();
        let output = std::process::Command::new("sh")
            .args(["-c", script])
            .current_dir(temporary.path())
            .env("ENTWINE_SITE_TOKEN", "test-secret-never-printed")
            .env("BITBUCKET_WORKSPACE", "workspace")
            .env("BITBUCKET_REPO_SLUG", bad)
            .output()
            .unwrap();
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("Invalid Bitbucket workspace or repository slug"));
        assert!(
            !stderr.contains("test-secret")
                && !String::from_utf8_lossy(&output.stdout).contains("test-secret")
        );
        assert!(!temporary.path().join("injected").exists());
    }
}
