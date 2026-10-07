use crate::{error, markdown::Parsed};
use entwine_core::*;
use percent_encoding::{percent_decode_str, utf8_percent_encode, NON_ALPHANUMERIC};
use pulldown_cmark::{CowStr, Event, Tag, TagEnd};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

type Targets = BTreeMap<String, (DocumentId, Route, Vec<String>)>;
const URL_SEGMENT: &percent_encoding::AsciiSet = &NON_ALPHANUMERIC
    .remove(b'.')
    .remove(b'-')
    .remove(b'_')
    .remove(b'~');

/// Generate a portable relative URL from one page route to a route or asset path.
pub(crate) fn relative_url(from: &str, to: &str) -> String {
    let from: Vec<_> = from
        .trim_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    let trailing = to.ends_with('/');
    let to: Vec<_> = to
        .trim_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    let shared = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let mut result = "../".repeat(from.len() - shared);
    result.push_str(
        &to[shared..]
            .iter()
            .map(|s| utf8_percent_encode(s, URL_SEGMENT).to_string())
            .collect::<Vec<_>>()
            .join("/"),
    );
    if result.is_empty() {
        return "./".into();
    }
    if trailing && !result.ends_with('/') {
        result.push('/');
    }
    result
}

fn normalize(source: &str, destination: &str) -> Result<String, String> {
    let decoded = percent_decode_str(destination)
        .decode_utf8()
        .map_err(|_| "Link path is not valid UTF-8".to_string())?;
    if decoded.contains('\\') || decoded.chars().any(|c| c.is_control() || c == ':') {
        return Err("Invalid link path".into());
    }
    let mut parts: Vec<_> = if decoded.starts_with('/') {
        Vec::new()
    } else {
        source
            .split('/')
            .rev()
            .skip(1)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    };
    for part in decoded.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err("Link traverses outside docs/".into());
                }
            }
            value => parts.push(value),
        }
    }
    Ok(parts.join("/"))
}

fn resolve(
    source: &str,
    route: &Route,
    destination: &str,
    targets: &Targets,
    assets: &BTreeSet<String>,
    image: bool,
) -> Result<(String, Option<DocumentId>), String> {
    if destination.starts_with("//") {
        return Ok((destination.into(), None));
    }
    if let Some((scheme, _)) = destination.split_once(':') {
        if ["http", "https", "mailto", "tel"].contains(&scheme.to_ascii_lowercase().as_str()) {
            return Ok((destination.into(), None));
        }
        return Err(format!("Unsafe or unsupported link scheme: {scheme}"));
    }
    let (without_anchor, anchor) = destination
        .split_once('#')
        .map_or((destination, None), |(p, a)| (p, Some(a)));
    let (path, query) = without_anchor
        .split_once('?')
        .map_or((without_anchor, None), |(p, q)| (p, Some(q)));
    let normalized = if path.is_empty() {
        source.into()
    } else {
        normalize(source, path)?
    };
    let target = targets
        .get(&normalized)
        .or_else(|| targets.get(&format!("{normalized}.md")))
        .or_else(|| targets.get(&format!("{}/index.md", normalized.trim_end_matches('/'))))
        .or_else(|| {
            // Authored routes win over generated case-equivalent directory indexes.
            if targets
                .keys()
                .any(|id| id.starts_with(&(normalized.clone() + "/")))
            {
                let directory = format!("/{}/", normalized.trim_matches('/'));
                targets
                    .values()
                    .find(|(_, route, _)| route.as_str().eq_ignore_ascii_case(&directory))
            } else {
                None
            }
        })
        .or_else(|| {
            if normalized.is_empty() {
                targets.get("index.md").or_else(|| targets.get("README.md"))
            } else {
                None
            }
        });
    let (mut href, id) = if !image {
        if let Some((id, target_route, headings)) = target {
            if let Some(anchor) = anchor.filter(|a| !a.is_empty()) {
                let decoded = percent_decode_str(anchor)
                    .decode_utf8()
                    .map_err(|_| "Anchor is not valid UTF-8".to_string())?;
                if !headings.iter().any(|h| h == &decoded) {
                    return Err(format!("Broken heading anchor #{anchor}: document {normalized} exists, but this anchor does not"));
                }
            }
            (
                relative_url(route.as_str(), target_route.as_str()),
                Some(id.clone()),
            )
        } else if normalized.is_empty()
            || targets
                .keys()
                .any(|id| id.starts_with(&(normalized.clone() + "/")))
        {
            if anchor.is_some_and(|a| !a.is_empty()) {
                return Err("Generated landing page has no heading anchors".into());
            }
            (
                relative_url(
                    route.as_str(),
                    &format!("/{}/", normalized.trim_matches('/')),
                ),
                None,
            )
        } else if assets.contains(&normalized) {
            (relative_url(route.as_str(), &normalized), None)
        } else {
            return Err(format!("Broken internal link {destination:?}: no document or asset resolves to docs/{normalized}"));
        }
    } else if assets.contains(&normalized) {
        (relative_url(route.as_str(), &normalized), None)
    } else {
        return Err(format!(
            "Broken image {destination:?}: no asset resolves to docs/{normalized}"
        ));
    };
    if let Some(query) = query {
        href.push('?');
        href.push_str(query);
    }
    if let Some(anchor) = anchor {
        href.push('#');
        href.push_str(anchor);
    }
    Ok((href, id))
}

pub(crate) type Artifacts = BTreeMap<String, (DocumentId, Route)>;

#[allow(clippy::too_many_arguments)]
pub(crate) fn links(
    parsed: &mut Parsed,
    targets: &Targets,
    assets: &BTreeSet<String>,
    docs: &Path,
    repository: &Path,
    source: Option<&RepositorySource>,
    artifacts: &Artifacts,
    publish_artifacts: bool,
    dependencies: &mut Vec<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (link, index) in parsed.document.links.iter_mut().zip(&parsed.link_events) {
        let reference = repository_reference(
            docs,
            repository,
            &parsed.document.id.0,
            &link.destination,
            dependencies,
        );
        match resolve(
            &parsed.document.id.0,
            &parsed.document.route,
            &link.destination,
            targets,
            assets,
            false,
        ) {
            Ok((href, target)) => {
                link.href = href;
                link.target = target;
            }
            Err(message) => {
                match reference {
                    Ok(Some(path)) => {
                        // A discovered agent artifact is knowledge, not a plain repository file.
                        if let Some((id, route)) = artifacts.get(&path) {
                            link.target = Some(id.clone());
                            if publish_artifacts {
                                link.href =
                                    relative_url(parsed.document.route.as_str(), route.as_str());
                                if let Some((_, fragment)) = link.destination.split_once('#') {
                                    link.href.push('#');
                                    link.href.push_str(fragment);
                                }
                                if let Some(Event::Start(Tag::Link { dest_url, .. })) =
                                    parsed.events.get_mut(*index)
                                {
                                    *dest_url = CowStr::from(link.href.clone());
                                }
                                continue;
                            }
                        }
                        let source = source
                            .filter(|s| s.files.as_ref().is_none_or(|files| files.contains(&path)));
                        link.href = source
                            .map(|s| source_url(s, &path, &link.destination))
                            .unwrap_or_default();
                        if link.target.is_none() {
                            parsed
                                .document
                                .repository_references
                                .push(RepositoryReference {
                                    path: path.clone(),
                                    source: parsed.document.id.clone(),
                                    line: link.line,
                                    destination: link.destination.clone(),
                                });
                        }
                        if source.is_none() {
                            parsed.events[*index] = Event::Html(CowStr::from(format!("<span class=\"repository-reference\" title=\"Repository file: {}\">", crate::render::escape(&path))));
                            if let Some(end) =
                                parsed.events.iter().enumerate().skip(index + 1).find_map(
                                    |(i, e)| matches!(e, Event::End(TagEnd::Link)).then_some(i),
                                )
                            {
                                parsed.events[end] = Event::Html(CowStr::from(format!(
                                    " <code>{}</code></span>",
                                    crate::render::escape(&path)
                                )));
                            }
                        }
                    }
                    Ok(None) => {
                        diagnostics.push(error(&parsed.document.id.0, Some(link.line), message));
                        link.href = "#".into();
                    }
                    Err(message) => {
                        diagnostics.push(error(&parsed.document.id.0, Some(link.line), message));
                        link.href = "#".into();
                    }
                }
            }
        }
        if let Some(Event::Start(Tag::Link { dest_url, .. })) = parsed.events.get_mut(*index) {
            *dest_url = CowStr::from(link.href.clone());
        }
    }
    for (index, line) in &parsed.image_events {
        if let Some(Event::Start(Tag::Image { dest_url, .. })) = parsed.events.get_mut(*index) {
            match resolve(
                &parsed.document.id.0,
                &parsed.document.route,
                dest_url,
                targets,
                assets,
                true,
            ) {
                Ok((href, _)) => *dest_url = CowStr::from(href),
                Err(message) => {
                    diagnostics.push(error(&parsed.document.id.0, Some(*line), message));
                    *dest_url = CowStr::from("#");
                }
            }
        }
    }
}

fn repository_reference(
    docs: &Path,
    repository: &Path,
    source: &str,
    destination: &str,
    dependencies: &mut Vec<String>,
) -> Result<Option<String>, String> {
    if destination.starts_with('/') || destination.contains(':') {
        return Ok(None);
    }
    let path = destination.split(['#', '?']).next().unwrap_or("");
    if path.is_empty() {
        return Ok(None);
    }
    let decoded = percent_decode_str(path)
        .decode_utf8()
        .map_err(|_| "Link path is not valid UTF-8")?;
    if decoded.contains('\\') || decoded.chars().any(|c| c.is_control() || c == ':') {
        return Err("Invalid repository reference path".into());
    }
    let relative = docs
        .strip_prefix(repository)
        .map_err(|_| "Documentation is outside the repository")?;
    let mut parts: Vec<String> = relative
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    parts.extend(
        source
            .split('/')
            .rev()
            .skip(1)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(str::to_string),
    );
    for part in decoded.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err("Link traverses outside the repository".into());
                }
            }
            value => parts.push(value.into()),
        }
    }
    let mut target = repository.to_path_buf();
    for part in &parts {
        target.push(part);
        if std::fs::symlink_metadata(&target).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err("Repository reference crosses a symbolic link; use regular files".into());
        }
    }
    if target.starts_with(docs) {
        return Ok(None);
    }
    dependencies.push(parts.join("/"));
    if target.is_dir() {
        return Err(format!("Repository reference points to a directory: {}; link to a regular file such as its README instead", parts.join("/")));
    }
    if !target.is_file() {
        return Err(format!(
            "Repository file does not exist: {}",
            parts.join("/")
        ));
    }
    Ok(Some(parts.join("/")))
}

pub(crate) fn source_url(source: &RepositorySource, path: &str, destination: &str) -> String {
    let mut url = source.file_base_url.clone();
    url.push_str(
        &path
            .split('/')
            .map(|p| utf8_percent_encode(p, URL_SEGMENT).to_string())
            .collect::<Vec<_>>()
            .join("/"),
    );
    if let Some((_, fragment)) = destination.split_once('#') {
        url.push('#');
        url.push_str(
            &utf8_percent_encode(
                &percent_decode_str(fragment).decode_utf8_lossy(),
                URL_SEGMENT,
            )
            .to_string(),
        );
    }
    url
}

/// Everything an agent artifact's links can resolve to, keyed by repository path.
pub(crate) struct ArtifactContext<'a> {
    pub documents: &'a BTreeMap<String, (DocumentId, Route)>,
    pub artifacts: &'a Artifacts,
    pub repository: &'a Path,
}

/// Resolve links inside an agent artifact. Unlike documentation, a broken link here is a
/// warning: these files are written for tools and often reference paths that come and go.
pub(crate) fn artifact_links(
    parsed: &mut Parsed,
    path: &str,
    context: &ArtifactContext,
    dependencies: &mut Vec<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let from = parsed.document.route.clone();
    let directory = path.rsplit_once('/').map_or("", |(d, _)| d).to_string();
    for (link, index) in parsed.document.links.iter_mut().zip(&parsed.link_events) {
        let destination = link.destination.clone();
        let mut href = destination.clone();
        let lowered = destination.to_ascii_lowercase();
        let external = destination.starts_with("//")
            || ["http:", "https:", "mailto:", "tel:"]
                .iter()
                .any(|scheme| lowered.starts_with(scheme));
        if external || destination.starts_with('#') || destination.is_empty() {
            // Left as written.
        } else if destination.contains(':') {
            diagnostics.push(crate::warning(
                &parsed.document.id.0,
                Some(link.line),
                format!("Unsupported link scheme in {destination:?}"),
            ));
            href = "#".into();
        } else {
            let (without_anchor, anchor) = destination
                .split_once('#')
                .map_or((destination.as_str(), None), |(p, a)| (p, Some(a)));
            let file = without_anchor.split('?').next().unwrap_or("");
            let resolved = percent_decode_str(file)
                .decode_utf8()
                .map_err(|_| "Link path is not valid UTF-8".to_string())
                .and_then(|decoded| {
                    let mut parts: Vec<&str> = if decoded.starts_with('/') {
                        Vec::new()
                    } else {
                        directory.split('/').filter(|p| !p.is_empty()).collect()
                    };
                    let decoded = decoded.into_owned();
                    for part in decoded.split('/') {
                        match part {
                            "" | "." => {}
                            ".." => {
                                if parts.pop().is_none() {
                                    return Err("Link traverses outside the repository".into());
                                }
                            }
                            value => parts.push(value),
                        }
                    }
                    Ok(parts.join("/"))
                });
            match resolved {
                Ok(target) if target.is_empty() => {}
                Ok(target) => {
                    let known = context
                        .documents
                        .get(&target)
                        .or_else(|| context.documents.get(&format!("{target}.md")))
                        .or_else(|| context.documents.get(&format!("{target}/index.md")))
                        .or_else(|| context.artifacts.get(&target));
                    if let Some((id, route)) = known {
                        link.target = Some(id.clone());
                        href = relative_url(from.as_str(), route.as_str());
                        if let Some(anchor) = anchor {
                            href.push('#');
                            href.push_str(anchor);
                        }
                    } else {
                        let mut file_path = context.repository.to_path_buf();
                        let mut crosses_symlink = false;
                        for part in target.split('/') {
                            file_path.push(part);
                            crosses_symlink |= std::fs::symlink_metadata(&file_path)
                                .is_ok_and(|m| m.file_type().is_symlink());
                        }
                        dependencies.push(target.clone());
                        if crosses_symlink {
                            diagnostics.push(crate::warning(
                                &parsed.document.id.0,
                                Some(link.line),
                                format!("Link crosses a symbolic link: {target}"),
                            ));
                        } else if file_path.is_file() {
                            parsed
                                .document
                                .repository_references
                                .push(RepositoryReference {
                                    path: target,
                                    source: parsed.document.id.clone(),
                                    line: link.line,
                                    destination: destination.clone(),
                                });
                        } else if !file_path.is_dir() {
                            diagnostics.push(crate::warning(
                                &parsed.document.id.0,
                                Some(link.line),
                                format!("Link target does not exist: {target}"),
                            ));
                        }
                        // A directory is a normal thing for an agent file to point at. Entwine
                        // models files, so it stays plain text without a diagnostic.
                        href = "#".into();
                    }
                }
                Err(message) => {
                    diagnostics.push(crate::warning(
                        &parsed.document.id.0,
                        Some(link.line),
                        message,
                    ));
                    href = "#".into();
                }
            }
        }
        link.href = href;
        if let Some(Event::Start(Tag::Link { dest_url, .. })) = parsed.events.get_mut(*index) {
            *dest_url = CowStr::from(link.href.clone());
        }
    }
    // Images in agent files are not published or resolved; show their alt text only.
    for (index, _) in &parsed.image_events {
        if let Some(Event::Start(Tag::Image { dest_url, .. })) = parsed.events.get_mut(*index) {
            *dest_url = CowStr::from("#");
        }
    }
}

/// Percent-encode one URL query or path component.
pub(crate) fn encode_component(value: &str) -> String {
    utf8_percent_encode(value, URL_SEGMENT).to_string()
}

/// `@path` imports in agent instruction files.
///
/// Several tools let an instruction file pull in another with `@relative/path` (for example
/// `CLAUDE.md` containing `@AGENTS.md`). That is an explicit reference, so Entwine records it as
/// one. Only tokens that resolve to a file that exists in the repository count; everything else
/// (`@scope/package`, `@mention`) is ordinary prose and is ignored without a diagnostic. Code
/// blocks and code spans are skipped. Entwine records the reference; it does not claim any tool
/// will load it.
pub(crate) fn instruction_imports(
    parsed: &mut Parsed,
    path: &str,
    context: &ArtifactContext,
    published: bool,
    dependencies: &mut Vec<String>,
) {
    let from = parsed.document.route.clone();
    let directory = path.rsplit_once('/').map_or("", |(d, _)| d).to_string();
    let content = parsed.document.content.clone();
    let mut fenced = false;
    for (index, line) in content.lines().enumerate() {
        if line.trim_start().starts_with("```") || line.trim_start().starts_with("~~~") {
            fenced = !fenced;
            continue;
        }
        if fenced || line.starts_with("    ") {
            continue;
        }
        for token in imports_in(line) {
            let Some(target) = resolve_import(&directory, token) else {
                continue;
            };
            let destination = format!("@{token}");
            if parsed
                .document
                .links
                .iter()
                .any(|l| l.destination == destination)
            {
                continue;
            }
            let known = context
                .documents
                .get(&target)
                .or_else(|| context.artifacts.get(&target));
            if let Some((id, route)) = known {
                if *id == parsed.document.id {
                    continue;
                }
                parsed.document.links.push(Link {
                    destination,
                    line: index + 1,
                    target: Some(id.clone()),
                    href: if published {
                        relative_url(from.as_str(), route.as_str())
                    } else {
                        String::new()
                    },
                });
                continue;
            }
            let mut file_path = context.repository.to_path_buf();
            let mut crosses_symlink = false;
            for part in target.split('/') {
                file_path.push(part);
                crosses_symlink |=
                    std::fs::symlink_metadata(&file_path).is_ok_and(|m| m.file_type().is_symlink());
            }
            if crosses_symlink || !file_path.is_file() {
                continue;
            }
            dependencies.push(target.clone());
            parsed
                .document
                .repository_references
                .push(RepositoryReference {
                    path: target,
                    source: parsed.document.id.clone(),
                    line: index + 1,
                    destination: destination.clone(),
                });
            parsed.document.links.push(Link {
                destination,
                line: index + 1,
                target: None,
                href: String::new(),
            });
        }
    }
}

/// Candidate `@path` tokens on one line, outside inline code, trailing punctuation removed.
fn imports_in(line: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut in_code = false;
    let mut offset = 0;
    for part in line.split('`') {
        if !in_code {
            for (at, _) in part.match_indices('@') {
                let before = part[..at].chars().next_back();
                if before.is_some_and(|c| !c.is_whitespace() && !"(\"'".contains(c)) {
                    continue; // e-mail addresses and glued text
                }
                let rest = &part[at + 1..];
                let end = rest
                    .find(|c: char| c.is_whitespace() || ")>]\"'`".contains(c))
                    .unwrap_or(rest.len());
                let token = rest[..end].trim_end_matches(['.', ',', ';', ':', '!', '?']);
                if !token.is_empty() {
                    found.push(token);
                }
            }
        }
        in_code = !in_code;
        offset += part.len() + 1;
    }
    let _ = offset;
    found
}

/// Normalize an import path relative to the importing file's directory.
fn resolve_import(directory: &str, token: &str) -> Option<String> {
    if token.starts_with('~') || token.starts_with('/') || token.contains(['\\', ':', '?', '#']) {
        return None;
    }
    let mut parts: Vec<&str> = directory.split('/').filter(|p| !p.is_empty()).collect();
    for part in token.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            value => parts.push(value),
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}
