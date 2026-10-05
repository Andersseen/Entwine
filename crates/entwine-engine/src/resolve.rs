use crate::{error, markdown::Parsed};
use entwine_core::*;
use percent_encoding::{percent_decode_str, utf8_percent_encode, NON_ALPHANUMERIC};
use pulldown_cmark::{CowStr, Event, Tag};
use std::collections::{BTreeMap, BTreeSet};

type Targets = BTreeMap<String, (DocumentId, Route, Vec<Heading>)>;
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
            if normalized.is_empty() {
                targets.get("index.md")
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
                if !headings.iter().any(|h| h.id == decoded) {
                    return Err(format!("Broken heading anchor #{anchor} in {normalized}"));
                }
            }
            (
                relative_url(route.as_str(), target_route.as_str()),
                Some(id.clone()),
            )
        } else if normalized.is_empty() {
            if anchor.is_some_and(|a| !a.is_empty()) {
                return Err("Generated landing page has no heading anchors".into());
            }
            (relative_url(route.as_str(), "/"), None)
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

pub(crate) fn links(
    parsed: &mut Parsed,
    targets: &Targets,
    assets: &BTreeSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (link, index) in parsed.document.links.iter_mut().zip(&parsed.link_events) {
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
                diagnostics.push(error(&parsed.document.id.0, Some(link.line), message));
                link.href = "#".into();
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
