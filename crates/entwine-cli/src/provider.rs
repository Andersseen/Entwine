//! Git remote inspection and provider classification. Never touches the network.
use std::{path::Path, process::Command};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Provider {
    GitHub,
    GitLab,
    Bitbucket,
    Unknown,
}
impl Provider {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::GitHub => "GitHub",
            Self::GitLab => "GitLab",
            Self::Bitbucket => "Bitbucket Cloud",
            Self::Unknown => "Unknown Git",
        }
    }
}

/// A parsed remote location. Credentials in URLs are discarded immediately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Remote {
    pub host: String,
    pub path: String,
}

/// Parse HTTPS, `ssh://`, `git://`, and scp-like (`git@host:owner/repo.git`) remotes.
pub(crate) fn parse_remote(url: &str) -> Option<Remote> {
    let url = url.trim();
    if url.is_empty() || url.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return None;
    }
    let (authority, path) = if let Some((scheme, rest)) = url.split_once("://") {
        if !["https", "http", "ssh", "git", "git+ssh", "ssh+git"]
            .contains(&scheme.to_ascii_lowercase().as_str())
        {
            return None;
        }
        rest.split_once('/')?
    } else {
        // scp-like syntax has no scheme, and its host ends at the first colon.
        let (authority, path) = url.split_once(':')?;
        if authority.contains('/') {
            return None;
        }
        (authority, path)
    };
    let host = authority.rsplit('@').next()?;
    // Strip an explicit port; bracketed IPv6 hosts are not forge hosts we classify.
    let host = if host.starts_with('[') {
        return None;
    } else {
        host.split(':').next()?
    };
    let host = host.to_ascii_lowercase();
    let path = path.trim_matches('/').trim_end_matches(".git").to_string();
    let valid_host = !host.is_empty()
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
    let valid_path = !path.is_empty()
        && path.split('/').all(|part| {
            !part.is_empty()
                && part != "."
                && part != ".."
                && part
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
        });
    (valid_host && valid_path).then_some(Remote { host, path })
}

/// Classify by host. Only well-known forges and unmistakable `gitlab.*` hosts are
/// recognized automatically; anything else needs an explicit `--provider`.
pub(crate) fn classify(host: &str) -> Provider {
    match host {
        "github.com" | "www.github.com" => Provider::GitHub,
        "gitlab.com" | "www.gitlab.com" => Provider::GitLab,
        "bitbucket.org" | "www.bitbucket.org" => Provider::Bitbucket,
        h if h.starts_with("gitlab.") => Provider::GitLab,
        _ => Provider::Unknown,
    }
}

/// What was learned from the repository, without network access.
#[derive(Debug)]
pub(crate) struct Repository {
    /// Repository top level, if inside a Git work tree.
    pub root: Option<std::path::PathBuf>,
    pub remote_name: Option<String>,
    pub remote: Option<Remote>,
    pub provider: Provider,
    /// Remote default branch when known locally, else the current branch.
    pub default_branch: Option<String>,
}

fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
}
fn safe_ref_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('-')
        && !name.contains("..")
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._/-".contains(c))
}

/// Order candidates predictably: the current branch's remote, `origin`, then the rest sorted.
pub(crate) fn order_remotes(mut names: Vec<String>, branch_remote: Option<&str>) -> Vec<String> {
    names.sort();
    names.dedup();
    let mut ordered = Vec::new();
    for preferred in [branch_remote, Some("origin")].into_iter().flatten() {
        if names.iter().any(|n| n == preferred) && !ordered.iter().any(|n| n == preferred) {
            ordered.push(preferred.to_string());
        }
    }
    let rest: Vec<_> = names.into_iter().filter(|n| !ordered.contains(n)).collect();
    ordered.extend(rest);
    ordered
}

pub(crate) fn inspect(project: &Path) -> Repository {
    let root = git(project, &["rev-parse", "--show-toplevel"]).map(std::path::PathBuf::from);
    let mut repository = Repository {
        root: root.clone(),
        remote_name: None,
        remote: None,
        provider: Provider::Unknown,
        default_branch: None,
    };
    let Some(root) = root else { return repository };
    let branch =
        git(&root, &["symbolic-ref", "--short", "-q", "HEAD"]).filter(|b| safe_ref_name(b));
    let branch_remote = branch
        .as_ref()
        .and_then(|b| git(&root, &["config", "--get", &format!("branch.{b}.remote")]))
        .filter(|r| safe_ref_name(r));
    let names: Vec<_> = git(&root, &["remote"])
        .map(|text| {
            text.lines()
                .map(str::to_string)
                .filter(|n| safe_ref_name(n))
                .collect()
        })
        .unwrap_or_default();
    let mut first = None;
    for name in order_remotes(names, branch_remote.as_deref()) {
        let Some(url) = git(&root, &["remote", "get-url", &name]) else {
            continue;
        };
        let Some(remote) = parse_remote(&url) else {
            continue;
        };
        let provider = classify(&remote.host);
        if first.is_none() {
            first = Some((name.clone(), remote.clone(), provider));
        }
        if provider != Provider::Unknown {
            first = Some((name, remote, provider));
            break;
        }
    }
    if let Some((name, remote, provider)) = first {
        repository.default_branch = git(
            &root,
            &[
                "symbolic-ref",
                "--short",
                "-q",
                &format!("refs/remotes/{name}/HEAD"),
            ],
        )
        .and_then(|b| b.strip_prefix(&format!("{name}/")).map(str::to_string))
        .filter(|b| safe_ref_name(b));
        repository.remote_name = Some(name);
        repository.remote = Some(remote);
        repository.provider = provider;
    }
    if repository.default_branch.is_none() {
        repository.default_branch = branch;
    }
    repository
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider(url: &str) -> Provider {
        classify(&parse_remote(url).unwrap().host)
    }

    #[test]
    fn recognizes_common_https_and_ssh_forms() {
        for (url, expected) in [
            ("https://github.com/foo/bar.git", Provider::GitHub),
            ("git@github.com:foo/bar.git", Provider::GitHub),
            ("ssh://git@github.com/foo/bar", Provider::GitHub),
            (
                "https://user:secret@github.com/foo/bar.git",
                Provider::GitHub,
            ),
            ("https://gitlab.com/foo/bar.git", Provider::GitLab),
            ("git@gitlab.com:foo/sub/bar.git", Provider::GitLab),
            ("https://gitlab.example.org/foo/bar.git", Provider::GitLab),
            ("https://bitbucket.org/foo/bar.git", Provider::Bitbucket),
            ("git@bitbucket.org:foo/bar.git", Provider::Bitbucket),
            (
                "ssh://git@bitbucket.org:22/foo/bar.git",
                Provider::Bitbucket,
            ),
            ("https://git.example.com/foo/bar.git", Provider::Unknown),
            ("git@code.internal:foo/bar.git", Provider::Unknown),
            (
                "https://bitbucket.corp.example/scm/foo/bar.git",
                Provider::Unknown,
            ),
            // Lookalikes must not be classified by suffix or substring.
            ("https://evilgithub.com/foo/bar.git", Provider::Unknown),
            (
                "https://github.com.evil.example/foo/bar.git",
                Provider::Unknown,
            ),
            ("https://notgitlab.com/foo/bar.git", Provider::Unknown),
        ] {
            assert_eq!(provider(url), expected, "{url}");
        }
    }

    #[test]
    fn credentials_are_dropped_and_garbage_is_rejected() {
        let remote = parse_remote("https://user:secret@github.com/foo/bar.git").unwrap();
        assert_eq!(
            remote,
            Remote {
                host: "github.com".into(),
                path: "foo/bar".into()
            }
        );
        for bad in [
            "",
            "   ",
            "/local/path/repo",
            "../repo",
            "file:///tmp/repo",
            "ftp://x.com/a",
            "https://",
            "https://github.com/",
            "host: with space",
            "a\nb:c/d",
        ] {
            assert!(parse_remote(bad).is_none(), "{bad:?}");
        }
    }

    #[test]
    fn remotes_are_ordered_predictably() {
        let names = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            order_remotes(names(&["zeta", "origin", "alpha"]), None),
            ["origin", "alpha", "zeta"]
        );
        assert_eq!(
            order_remotes(names(&["origin", "fork"]), Some("fork")),
            ["fork", "origin"]
        );
        assert_eq!(
            order_remotes(names(&["b", "a"]), Some("missing")),
            ["a", "b"]
        );
    }
}

/// Immutable file links use the local commit, never a branch or remote path as
/// filesystem input. Unknown hosts receive no guessed URL.
pub(crate) fn source(repository: &Repository) -> Option<entwine_core::RepositorySource> {
    let remote = repository.remote.as_ref()?;
    let commit = git(repository.root.as_ref()?, &["rev-parse", "HEAD"])?;
    if commit.len() < 40 || !commit.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let mut source = source_at(remote, repository.provider, &commit)?;
    source.files = Some(
        git(
            repository.root.as_ref()?,
            &["ls-tree", "-rz", "--name-only", "HEAD"],
        )?
        .split('\0')
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect(),
    );
    Some(source)
}
fn source_at(
    remote: &Remote,
    provider: Provider,
    reference: &str,
) -> Option<entwine_core::RepositorySource> {
    if remote.path.split('/').any(|p| {
        p.is_empty()
            || p == "."
            || p == ".."
            || !p
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
    }) {
        return None;
    }
    let ref_url =
        percent_encoding::utf8_percent_encode(reference, percent_encoding::NON_ALPHANUMERIC)
            .to_string();
    let segment = match provider {
        Provider::GitHub => "blob",
        Provider::GitLab => "-/blob",
        Provider::Bitbucket => "src",
        Provider::Unknown => return None,
    };
    Some(entwine_core::RepositorySource {
        files: None,
        file_base_url: format!(
            "https://{}/{}/{segment}/{ref_url}/",
            remote.host, remote.path
        ),
    })
}
#[cfg(test)]
mod source_tests {
    use super::*;
    #[test]
    fn source_urls_are_portable_and_never_contain_remote_credentials() {
        for (remote, segment) in [
            ("https://user:secret@github.com/org/repo.git", "blob"),
            ("git@github.com:org/repo.git", "blob"),
            ("git@gitlab.com:group/sub/repo.git", "-/blob"),
            ("ssh://git@bitbucket.org:22/team/repo.git", "src"),
            ("https://gitlab.internal/group/repo.git", "-/blob"),
        ] {
            let r = parse_remote(remote).unwrap();
            let s = source_at(&r, classify(&r.host), "feature/a?#").unwrap();
            assert!(s
                .file_base_url
                .contains(&format!("/{segment}/feature%2Fa%3F%23/")));
            assert!(!s.file_base_url.contains("secret"));
        }
        for remote in [
            "https://unknown.example/org/repo",
            "https://github.com/org/../repo",
            "https://github.com/org/repo?token=secret",
            "https://github.com/org/%2e%2e/repo",
            "https://github.com/org/repo#secret",
        ] {
            if let Some(r) = parse_remote(remote) {
                assert!(source_at(&r, classify(&r.host), "main").is_none());
            }
        }
    }
}
