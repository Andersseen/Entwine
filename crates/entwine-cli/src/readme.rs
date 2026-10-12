//! `entwine setup --readme-link`: an opt-in, reversible documentation link in the
//! repository README. Only a clearly marked managed block is ever written; everything
//! outside it is preserved byte for byte. Nothing here touches the network, and no CI
//! workflow ever runs this code.
use crate::provider::{Provider, Remote, Repository};
use std::{
    fs, io,
    io::Write,
    path::{Path, PathBuf},
};

pub(crate) const START: &str = "<!-- entwine:docs-link:start -->";
pub(crate) const END: &str = "<!-- entwine:docs-link:end -->";
const FILE: &str = "README.md";

/// Where the documentation URL came from; it decides how the result is worded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum UrlSource {
    Explicit,
    GitHubPages,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SiteUrl {
    pub url: String,
    pub source: UrlSource,
}

/// Validate a user-supplied documentation URL: credential-free HTTPS only.
pub(crate) fn parse_site_url(input: &str) -> Result<String, String> {
    let input = input.trim();
    let rest = input.strip_prefix("https://").ok_or_else(|| {
        format!("--site-url must be an https:// URL (got {input:?}); other schemes are refused")
    })?;
    if rest.is_empty()
        || input
            .chars()
            .any(|c| c.is_control() || c.is_whitespace() || "<>\"'`()[]{}\\|^".contains(c))
        || !input.is_ascii()
    {
        return Err(format!(
            "--site-url {input:?} contains characters that cannot be safely embedded in Markdown"
        ));
    }
    if input.contains('?') || input.contains('#') {
        return Err("--site-url must not contain a query string or fragment".into());
    }
    let (authority, path) = rest.split_once('/').map_or((rest, ""), |(a, p)| (a, p));
    if authority.contains('@') {
        return Err("--site-url must not embed credentials".into());
    }
    let (host, port) = authority
        .split_once(':')
        .map_or((authority, None), |(h, p)| (h, Some(p)));
    let host_ok = !host.is_empty()
        && !host.starts_with(['.', '-'])
        && !host.ends_with(['.', '-'])
        && !host.contains("..")
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || ".-".contains(c));
    let port_ok = port.is_none_or(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()));
    if !host_ok || !port_ok {
        return Err(format!("--site-url has an invalid host: {authority:?}"));
    }
    if path.split('/').any(|part| part == "." || part == "..") {
        return Err("--site-url must not contain `.` or `..` path segments".into());
    }
    let mut url = format!("https://{}", host.to_ascii_lowercase());
    if let Some(port) = port {
        url.push(':');
        url.push_str(port);
    }
    url.push('/');
    let path = path.trim_end_matches('/');
    if !path.is_empty() {
        url.push_str(path);
        // A trailing slash keeps relative links on static hosts correct.
        if !path.rsplit('/').next().unwrap_or("").contains('.') {
            url.push('/');
        }
    }
    Ok(url)
}

/// The GitHub project Pages URL, only when every GitHub remote names the same repository.
pub(crate) fn derive_github_pages(repository: &Repository) -> Result<SiteUrl, String> {
    if repository.provider != Provider::GitHub {
        return Err(format!(
            "{} has no documentation URL Entwine can derive safely; pass --site-url https://…",
            match repository.provider {
                Provider::GitLab =>
                    "GitLab Pages URLs depend on namespace and instance settings, so",
                Provider::Bitbucket => "Bitbucket static hosting depends on workspace settings, so",
                _ => "This remote",
            }
        ));
    }
    let github: Vec<&Remote> = repository
        .remotes
        .iter()
        .filter(|r| matches!(r.host.as_str(), "github.com" | "www.github.com"))
        .collect();
    let first = github.first().ok_or_else(|| {
        "The GitHub provider was chosen explicitly, but no github.com remote exists; pass --site-url https://…".to_string()
    })?;
    if github
        .iter()
        .any(|r| !r.path.eq_ignore_ascii_case(&first.path))
    {
        return Err("Several GitHub remotes name different repositories (a fork?), so the Pages URL is ambiguous; pass --site-url https://…".into());
    }
    let parts: Vec<&str> = first.path.split('/').collect();
    let [owner, name] = parts.as_slice() else {
        return Err(format!(
            "Remote path {:?} is not a single owner/repository pair; pass --site-url https://…",
            first.path
        ));
    };
    let host = format!("{}.github.io", owner.to_ascii_lowercase());
    let url = if name.eq_ignore_ascii_case(&host) {
        format!("https://{host}/")
    } else {
        format!("https://{host}/{name}/")
    };
    Ok(SiteUrl {
        url,
        source: UrlSource::GitHubPages,
    })
}

fn eol(text: &str) -> &'static str {
    if text.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

fn block(url: &str, eol: &str) -> String {
    [
        START,
        "<!-- Managed by `entwine setup --readme-link`; edits between these markers are overwritten. -->",
        &format!("**Documentation:** <{url}>"),
        END,
    ]
    .join(eol)
}

/// Byte range of the managed block (including both markers), or an error for markers
/// that are duplicated, unpaired, reversed, or not on their own lines.
fn find_block(text: &str) -> Result<Option<(usize, usize)>, String> {
    let starts: Vec<_> = text.match_indices(START).map(|(i, _)| i).collect();
    let ends: Vec<_> = text.match_indices(END).map(|(i, _)| i).collect();
    match (starts.as_slice(), ends.as_slice()) {
        ([], []) => Ok(None),
        ([start], [end]) if start < end => {
            let own_line = |at: usize, len: usize| {
                let before = text[..at].rsplit('\n').next().unwrap_or("");
                let after = text[at + len..].split('\n').next().unwrap_or("");
                before.trim().is_empty() && after.trim().is_empty()
            };
            if own_line(*start, START.len()) && own_line(*end, END.len()) {
                Ok(Some((*start, end + END.len())))
            } else {
                Err("the Entwine markers must each sit on a line of their own".into())
            }
        }
        ([_], [_]) => Err("the end marker appears before the start marker".into()),
        _ => Err(format!(
            "found {} start and {} end markers; expected exactly one pair",
            starts.len(),
            ends.len()
        )),
    }
}

/// Documentation links the humans who maintain the README already wrote, outside the block.
fn human_docs_links(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut fenced = false;
    let mut previous = "";
    for line in text.lines() {
        let context = format!("{previous} {line}");
        previous = line;
        if line.trim_start().starts_with("```") || line.trim_start().starts_with("~~~") {
            fenced = !fenced;
        }
        if fenced {
            continue;
        }
        let lowered = context.to_ascii_lowercase();
        if !["documentation", "docs", "handbook", "guide"]
            .iter()
            .any(|word| lowered.contains(word))
        {
            continue;
        }
        let mut rest = line;
        while let Some(at) = rest.find("https://") {
            let candidate = &rest[at..];
            let end = candidate
                .find(|c: char| c.is_whitespace() || "<>\")]`'".contains(c))
                .unwrap_or(candidate.len());
            found.push(
                candidate[..end]
                    .trim_end_matches(['.', ',', ';'])
                    .to_string(),
            );
            rest = &candidate[end..];
        }
    }
    found
}

fn mentions_site(text: &str, url: &str) -> bool {
    // Same site: scheme, host case, and trailing slash do not matter; deeper pages count.
    let target = url.trim_end_matches('/').to_ascii_lowercase();
    let lowered = text.to_ascii_lowercase();
    lowered.match_indices(&target).any(|(at, _)| {
        matches!(
            lowered.as_bytes().get(at + target.len()),
            None | Some(
                b'/' | b'>'
                    | b')'
                    | b']'
                    | b'"'
                    | b'\''
                    | b' '
                    | b'\n'
                    | b'\r'
                    | b'`'
                    | b'.'
                    | b','
                    | b'#'
                    | b'?'
            )
        )
    })
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// Nothing to write.
    Unchanged(String),
    Write {
        content: String,
        summary: String,
        diff: Vec<String>,
    },
    /// A human decision is needed; nothing is written.
    Manual(String),
}

/// Decide the README result for the given text. Pure, so it is easy to test exhaustively.
pub(crate) fn plan_text(text: &str, site: Option<&SiteUrl>) -> Result<Outcome, String> {
    let existing = find_block(text).map_err(|e| format!("README.md is not changed: {e}"))?;
    let (before, after) = existing.map_or((text, ""), |(s, e)| (&text[..s], &text[e..]));
    let outside = format!("{before}{after}");
    let Some(site) = site else {
        return Ok(match (existing, human_docs_links(&outside).first()) {
            (Some(_), _) => Outcome::Unchanged(
                "README.md already contains the Entwine managed block; pass --site-url to change it".into(),
            ),
            (None, Some(link)) => Outcome::Unchanged(format!(
                "README.md already links documentation ({link}); no managed block needed"
            )),
            (None, None) => Outcome::Manual(
                "No documentation URL could be determined. Pass --site-url https://…".into(),
            ),
        });
    };
    let new_block = block(&site.url, eol(text));
    if let Some((start, end)) = existing {
        if text[start..end] == new_block {
            return Ok(Outcome::Unchanged(format!(
                "README.md already has the managed documentation link to {}",
                site.url
            )));
        }
        let content = format!("{}{new_block}{}", &text[..start], &text[end..]);
        let mut diff: Vec<String> = text[start..end].lines().map(|l| format!("- {l}")).collect();
        diff.extend(new_block.lines().map(|l| format!("+ {l}")));
        return Ok(Outcome::Write {
            content,
            summary: format!("update the managed documentation link to {}", site.url),
            diff,
        });
    }
    if mentions_site(&outside, &site.url) {
        return Ok(Outcome::Unchanged(format!(
            "README.md already links {}; nothing added",
            site.url
        )));
    }
    if let Some(link) = human_docs_links(&outside).first() {
        return Ok(Outcome::Manual(format!(
            "README.md already links documentation at {link}, which is not {}. Not changed; edit the README yourself if the link is outdated",
            site.url
        )));
    }
    let eol = eol(text);
    let (offset, lead) = insertion_point(text);
    let tail = &text[offset..];
    let mut inserted = String::new();
    if !text[..offset].is_empty() && !text[..offset].ends_with('\n') {
        inserted.push_str(eol);
    }
    inserted.push_str(lead);
    inserted.push_str(&new_block);
    inserted.push_str(eol);
    if !tail.is_empty() && !tail.starts_with('\n') && !tail.starts_with("\r\n") {
        inserted.push_str(eol);
    }
    let content = format!("{}{inserted}{tail}", &text[..offset]);
    Ok(Outcome::Write {
        content,
        summary: format!("add a managed documentation link to {}", site.url),
        diff: new_block.lines().map(|l| format!("+ {l}")).collect(),
    })
}

/// Just after the first level-1 heading outside a code fence, else the very top.
/// Returns the byte offset and any blank-line lead-in to insert.
fn insertion_point(text: &str) -> (usize, &'static str) {
    let eol = eol(text);
    let lead = if eol == "\r\n" { "\r\n" } else { "\n" };
    let mut offset = 0;
    let mut fenced = false;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.trim_start().starts_with("```") || trimmed.trim_start().starts_with("~~~") {
            fenced = !fenced;
        }
        offset += line.len();
        if !fenced && trimmed.starts_with("# ") {
            return (offset, lead);
        }
    }
    (0, "")
}

/// Write `content` over `path` atomically: a same-directory temporary file is flushed and
/// renamed into place, so a crash leaves either the old or the new README, never a mix.
fn replace_atomically(path: &Path, content: &str) -> io::Result<()> {
    let directory = path.parent().unwrap_or(Path::new("."));
    let mut file = tempfile::Builder::new()
        .prefix(".README.md.entwine-")
        .tempfile_in(directory)?;
    file.write_all(content.as_bytes())?;
    file.as_file().sync_all()?;
    fs::set_permissions(file.path(), fs::metadata(path)?.permissions())?;
    // The target must still be the regular file inspected earlier.
    if fs::symlink_metadata(path)?.file_type().is_symlink() {
        return Err(io::Error::other("README.md became a symbolic link"));
    }
    file.persist(path).map_err(|e| e.error)?;
    Ok(())
}

pub(crate) struct Request<'a> {
    pub root: &'a Path,
    pub repository: &'a Repository,
    pub site_url: Option<&'a str>,
    pub dry_run: bool,
}

fn readme_path(root: &Path) -> Result<Option<PathBuf>, String> {
    let path = root.join(FILE);
    match fs::symlink_metadata(&path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            Err("README.md is a symbolic link; Entwine will not write through it".into())
        }
        Ok(meta) if !meta.is_file() => Err("README.md is not a regular file".into()),
        Ok(_) => Ok(Some(path)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("Cannot inspect README.md: {e}")),
    }
}

/// Run the README step and print its result. Errors leave the file untouched.
pub(crate) fn run(request: &Request) -> Result<(), Box<dyn std::error::Error>> {
    println!("\nREADME documentation link");
    let site = match request.site_url {
        Some(url) => Some(SiteUrl {
            url: parse_site_url(url)?,
            source: UrlSource::Explicit,
        }),
        None => match derive_github_pages(request.repository) {
            Ok(site) => Some(site),
            Err(reason) => {
                println!("○ {reason}");
                None
            }
        },
    };
    let Some(path) = readme_path(request.root)? else {
        println!(
            "○ No README.md at the repository root; nothing to link (Entwine does not create one)"
        );
        return Ok(());
    };
    let text = fs::read_to_string(&path)
        .map_err(|e| format!("Cannot read README.md as UTF-8 text: {e}"))?;
    if let Some(site) = &site {
        match site.source {
            UrlSource::GitHubPages => println!(
                "○ Derived {} from the GitHub remote. Entwine has not verified that it is live; it works after the first Pages deployment.",
                site.url
            ),
            UrlSource::Explicit => println!("○ Using {} from --site-url (not verified)", site.url),
        }
    }
    match plan_text(&text, site.as_ref())? {
        Outcome::Unchanged(message) => println!("✓ {message}"),
        Outcome::Manual(message) => println!("⚠ {message}"),
        Outcome::Write {
            content,
            summary,
            diff,
        } => {
            if request.dry_run {
                println!("Would {summary} in README.md:");
                for line in &diff {
                    println!("  {line}");
                }
            } else {
                replace_atomically(&path, &content)?;
                println!("✓ README.md: {summary}");
                println!("  Review and commit it yourself; Entwine never commits or pushes.");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site(url: &str) -> SiteUrl {
        SiteUrl {
            url: url.into(),
            source: UrlSource::Explicit,
        }
    }
    fn written(outcome: Outcome) -> String {
        match outcome {
            Outcome::Write { content, .. } => content,
            other => panic!("expected a write, got {other:?}"),
        }
    }

    #[test]
    fn site_urls_are_normalized_and_unsafe_ones_rejected() {
        assert_eq!(
            parse_site_url("https://Docs.Example.com").unwrap(),
            "https://docs.example.com/"
        );
        assert_eq!(
            parse_site_url("https://x.github.io/Strata").unwrap(),
            "https://x.github.io/Strata/"
        );
        assert_eq!(
            parse_site_url("https://example.com:8443/a/b/").unwrap(),
            "https://example.com:8443/a/b/"
        );
        for bad in [
            "http://example.com/",
            "javascript:alert(1)",
            "ftp://example.com/",
            "https://user:pw@example.com/",
            "https://user@example.com/",
            "https://example.com/a b",
            "https://example.com/?q=1",
            "https://example.com/#x",
            "https://example.com/)(",
            "https://example.com/<x>",
            "https://example.com/../x",
            "https://exa mple.com/",
            "https:///x",
            "https://-bad.com/",
            "example.com",
            "",
        ] {
            assert!(parse_site_url(bad).is_err(), "{bad}");
        }
    }

    fn repository(provider: Provider, remotes: &[(&str, &str)]) -> Repository {
        Repository {
            root: None,
            remote_name: None,
            remote: None,
            provider,
            remotes: remotes
                .iter()
                .map(|(host, path)| Remote {
                    host: host.to_string(),
                    path: path.to_string(),
                })
                .collect(),
            default_branch: None,
        }
    }

    #[test]
    fn github_pages_urls_keep_repository_casing_and_are_never_guessed_elsewhere() {
        let derived = |r: &Repository| derive_github_pages(r).map(|s| s.url);
        assert_eq!(
            derived(&repository(
                Provider::GitHub,
                &[("github.com", "Andersseen/Strata")]
            ))
            .unwrap(),
            "https://andersseen.github.io/Strata/"
        );
        assert_eq!(
            derived(&repository(
                Provider::GitHub,
                &[("github.com", "Octo/octo.github.io")]
            ))
            .unwrap(),
            "https://octo.github.io/"
        );
        // Same repository through two remotes is unambiguous; a fork is not.
        assert!(derived(&repository(
            Provider::GitHub,
            &[("github.com", "a/b"), ("github.com", "A/B")]
        ))
        .is_ok());
        assert!(derived(&repository(
            Provider::GitHub,
            &[("github.com", "a/b"), ("github.com", "fork/b")]
        ))
        .is_err());
        assert!(derived(&repository(Provider::GitHub, &[("github.com", "a/b/c")])).is_err());
        assert!(derived(&repository(
            Provider::GitHub,
            &[("ghe.corp.example", "a/b")]
        ))
        .is_err());
        assert!(derived(&repository(Provider::GitLab, &[("gitlab.com", "a/b")])).is_err());
        assert!(derived(&repository(
            Provider::Bitbucket,
            &[("bitbucket.org", "a/b")]
        ))
        .is_err());
        assert!(derived(&repository(
            Provider::Unknown,
            &[("git.example.com", "a/b")]
        ))
        .is_err());
    }

    #[test]
    fn inserts_after_the_title_and_preserves_everything_else() {
        let text = "<p align=\"center\">logo</p>\n\n# Title\n\nIntro.\n\n```\n# not a title\n```\n";
        let out = written(plan_text(text, Some(&site("https://d.example/"))).unwrap());
        assert!(out.starts_with(
            "<p align=\"center\">logo</p>\n\n# Title\n\n<!-- entwine:docs-link:start -->"
        ));
        assert!(out.contains(
            "**Documentation:** <https://d.example/>\n<!-- entwine:docs-link:end -->\n\nIntro."
        ));
        // Removing the block restores the original exactly.
        let (start, end) = find_block(&out).unwrap().unwrap();
        let restored = format!("{}{}", &out[..start], &out[end..]);
        let mut collapsed = restored;
        while collapsed.contains("\n\n\n") {
            collapsed = collapsed.replace("\n\n\n", "\n\n");
        }
        assert_eq!(collapsed, text);
    }

    #[test]
    fn heading_without_following_blank_line_and_missing_heading() {
        let out = written(plan_text("# T\ntext\n", Some(&site("https://d.example/"))).unwrap());
        assert_eq!(
            out,
            format!("# T\n\n{}\n\ntext\n", block("https://d.example/", "\n"))
        );
        let out = written(plan_text("no heading\n", Some(&site("https://d.example/"))).unwrap());
        assert!(out.starts_with(START) && out.ends_with("no heading\n"));
        let out = written(plan_text("", Some(&site("https://d.example/"))).unwrap());
        assert_eq!(out, format!("{}\n", block("https://d.example/", "\n")));
        let out = written(plan_text("# T", Some(&site("https://d.example/"))).unwrap());
        assert!(out.starts_with("# T\n\n") && out.ends_with(&format!("{END}\n")));
    }

    #[test]
    fn crlf_files_stay_crlf() {
        let text = "# T\r\n\r\nBody\r\n";
        let out = written(plan_text(text, Some(&site("https://d.example/"))).unwrap());
        assert!(!out.replace("\r\n", "").contains('\n'), "{out:?}");
        assert!(out.ends_with("Body\r\n"));
    }

    #[test]
    fn second_run_is_a_no_op_and_changes_only_the_block() {
        let first =
            written(plan_text("# T\n\nBody\n", Some(&site("https://one.example/"))).unwrap());
        assert!(matches!(
            plan_text(&first, Some(&site("https://one.example/"))).unwrap(),
            Outcome::Unchanged(_)
        ));
        let updated = written(plan_text(&first, Some(&site("https://two.example/"))).unwrap());
        assert_eq!(updated, first.replace("one.example", "two.example"));
        // No URL known: the block is left alone.
        assert!(matches!(
            plan_text(&first, None).unwrap(),
            Outcome::Unchanged(_)
        ));
    }

    #[test]
    fn existing_human_links_are_recognized_without_duplication() {
        // Condensed from Andersseen/Strata's README.
        let strata = "# Strata\n\n## Documentation\n\nTechnical documentation lives as Markdown in [`docs/`](./docs/README.md). It is compiled by\n<https://andersseen.github.io/Strata/>.\n";
        let correct = site("https://andersseen.github.io/Strata/");
        assert!(matches!(
            plan_text(strata, Some(&correct)).unwrap(),
            Outcome::Unchanged(_)
        ));
        assert!(matches!(
            plan_text(strata, None).unwrap(),
            Outcome::Unchanged(_)
        ));
        // Deeper page, other casing, http scheme in the README: still the same site.
        for variant in [
            "# T\n\nDocs: [site](https://Andersseen.github.io/Strata/guide/)\n",
            "# T\n\nSee <https://andersseen.github.io/Strata>\n",
        ] {
            assert!(
                matches!(
                    plan_text(variant, Some(&correct)).unwrap(),
                    Outcome::Unchanged(_)
                ),
                "{variant}"
            );
        }
        // A different documentation URL is surfaced, never overwritten or duplicated.
        let other = "# T\n\nDocumentation: https://old.example/docs/\n";
        assert!(matches!(
            plan_text(other, Some(&correct)).unwrap(),
            Outcome::Manual(_)
        ));
        // A prefix of another host is not the same site.
        let similar = "# T\n\nhttps://andersseen.github.io/Strata-extra/\n";
        assert!(matches!(
            plan_text(similar, Some(&correct)).unwrap(),
            Outcome::Write { .. }
        ));
        // Links inside code fences are examples, not documentation links.
        let fenced = "# T\n\n```\ndocs: https://elsewhere.example/\n```\n";
        assert!(matches!(
            plan_text(fenced, Some(&correct)).unwrap(),
            Outcome::Write { .. }
        ));
    }

    #[test]
    fn malformed_duplicate_and_conflicting_markers_are_rejected() {
        let url = site("https://d.example/");
        let good = block("https://d.example/", "\n");
        for bad in [
            format!("# T\n\n{START}\nno end\n"),
            format!("# T\n\n{END}\nno start\n"),
            format!("# T\n\n{END}\n{START}\n"),
            format!("# T\n\n{good}\n\n{good}\n"),
            format!("# T\n\n{START}\n{START}\n{END}\n"),
            format!("# T\n\n{START}\n{END}\n{END}\n"),
            format!("# T\n\ninline {START} marker {END} on one line\n"),
        ] {
            let error = plan_text(&bad, Some(&url)).unwrap_err();
            assert!(error.contains("not changed"), "{error}");
            assert!(plan_text(&bad, None).is_err());
        }
    }

    #[test]
    fn without_a_url_nothing_is_invented() {
        assert!(matches!(
            plan_text("# T\n\nBody\n", None).unwrap(),
            Outcome::Manual(_)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_and_non_file_readmes_are_refused_and_writes_are_atomic() {
        let temp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("real.md"), "# Real\n").unwrap();
        std::os::unix::fs::symlink(
            outside.path().join("real.md"),
            temp.path().join("README.md"),
        )
        .unwrap();
        assert!(readme_path(temp.path()).is_err());
        fs::remove_file(temp.path().join("README.md")).unwrap();
        fs::create_dir(temp.path().join("README.md")).unwrap();
        assert!(readme_path(temp.path()).is_err());
        fs::remove_dir(temp.path().join("README.md")).unwrap();
        assert_eq!(readme_path(temp.path()).unwrap(), None);
        let path = temp.path().join("README.md");
        fs::write(&path, "# Old\n").unwrap();
        replace_atomically(&path, "# New\n").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "# New\n");
        let leftovers: Vec<_> = fs::read_dir(temp.path()).unwrap().flatten().collect();
        assert_eq!(leftovers.len(), 1, "no temporary file may remain");
        assert_eq!(
            fs::read_to_string(outside.path().join("real.md")).unwrap(),
            "# Real\n"
        );
    }
}
