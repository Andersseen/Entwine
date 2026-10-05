use crate::error;
use entwine_core::*;
use pulldown_cmark::{html, CowStr, Event, Options, Parser, Tag, TagEnd};
use std::collections::BTreeSet;

pub(crate) struct Parsed {
    pub document: Document,
    pub events: Vec<Event<'static>>,
    pub link_events: Vec<usize>,
    pub image_events: Vec<(usize, usize)>,
}

fn frontmatter<'a>(
    source: &str,
    text: &'a str,
    diagnostics: &mut Vec<Diagnostic>,
) -> (DocumentMetadata, &'a str, usize) {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    if text.lines().next() != Some("---") {
        return (DocumentMetadata::default(), text, 0);
    }
    let mut offset = 0;
    let mut closing = None;
    for (i, line) in text.split_inclusive('\n').enumerate() {
        if i > 0 && line.trim_end_matches(['\r', '\n']) == "---" {
            closing = Some((offset, offset + line.len(), i + 1));
            break;
        }
        offset += line.len();
    }
    let Some((end, start, lines)) = closing else {
        diagnostics.push(error(
            source,
            Some(1),
            "Unclosed frontmatter: add a closing --- line",
        ));
        return (DocumentMetadata::default(), text, 0);
    };
    let opening = text.find('\n').map_or(text.len(), |p| p + 1);
    let mut metadata = DocumentMetadata::default();
    match serde_yaml::from_str::<serde_yaml::Value>(&text[opening..end]) {
        Ok(serde_yaml::Value::Mapping(mapping)) => {
            for (key, value) in mapping {
                let Some(key) = key.as_str() else {
                    diagnostics.push(error(source, Some(2), "Frontmatter keys must be strings"));
                    continue;
                };
                if !["title", "type", "status"].contains(&key) {
                    continue;
                }
                let Some(value) = value
                    .as_str()
                    .filter(|s| !s.trim().is_empty() && !s.chars().any(char::is_control))
                else {
                    diagnostics.push(error(
                        source,
                        Some(2),
                        format!("Frontmatter {key} must be a non-empty single-line string"),
                    ));
                    continue;
                };
                match key {
                    "title" => metadata.title = Some(value.into()),
                    "type" => metadata.kind = Some(value.into()),
                    "status" => metadata.status = Some(value.into()),
                    _ => {}
                }
            }
        }
        Ok(serde_yaml::Value::Null) => {}
        Ok(_) => diagnostics.push(error(source, Some(2), "Frontmatter must be a YAML mapping")),
        Err(e) => diagnostics.push(error(
            source,
            e.location().map(|l| l.line() + 1),
            format!("Invalid frontmatter: {e}"),
        )),
    }
    (metadata, &text[start..], lines)
}

pub(crate) fn parse(
    source: &str,
    route: Route,
    text: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Parsed {
    let (metadata, body, line_offset) = frontmatter(source, text, diagnostics);
    let mut events = Vec::new();
    let mut headings = Vec::new();
    let mut links = Vec::new();
    let mut link_events = Vec::new();
    let mut image_events = Vec::new();
    let mut heading: Option<(usize, u8, String)> = None;
    let mut used_ids = BTreeSet::new();
    for (event, range) in Parser::new_ext(
        body,
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS,
    )
    .into_offset_iter()
    {
        let line = line_offset + 1 + body[..range.start].bytes().filter(|b| *b == b'\n').count();
        match &event {
            Event::Start(Tag::Heading { level, .. }) => {
                heading = Some((events.len(), *level as u8, String::new()))
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some((_, _, title)) = &mut heading {
                    title.push_str(text);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some((_, _, title)) = &mut heading {
                    title.push(' ');
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some((start, level, title)) = heading.take() {
                    let base = slug(&title);
                    let mut id = base.clone();
                    let mut suffix = 2;
                    while !used_ids.insert(id.clone()) {
                        id = format!("{base}-{suffix}");
                        suffix += 1;
                    }
                    if let Some(Event::Start(Tag::Heading { id: slot, .. })) = events.get_mut(start)
                    {
                        *slot = Some(CowStr::from(id.clone()));
                    }
                    headings.push(Heading {
                        level,
                        text: title,
                        id,
                    });
                }
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                link_events.push(events.len());
                links.push(Link {
                    destination: dest_url.to_string(),
                    href: dest_url.to_string(),
                    line,
                    target: None,
                });
            }
            Event::Start(Tag::Image { .. }) => image_events.push((events.len(), line)),
            _ => {}
        }
        // Raw HTML is displayed as text; scripts and arbitrary author markup never execute.
        events.push(match event {
            Event::Html(text) | Event::InlineHtml(text) => Event::Text(text.into_static()),
            other => other.into_static(),
        });
    }
    let title = document_title(&metadata, &headings, source);
    Parsed {
        document: Document {
            id: DocumentId(source.into()),
            route,
            title,
            metadata,
            headings,
            links,
            content: body.into(),
            html: String::new(),
        },
        events,
        link_events,
        image_events,
    }
}

fn slug(text: &str) -> String {
    let mut result = String::new();
    for c in text.to_lowercase().chars() {
        if c.is_alphanumeric() || c == '_' {
            result.push(c);
        } else if (c.is_whitespace() || c == '-') && !result.is_empty() && !result.ends_with('-') {
            result.push('-');
        }
    }
    let result = result.trim_end_matches('-');
    if result.is_empty() {
        "section".into()
    } else {
        result.into()
    }
}

pub(crate) fn finish(mut parsed: Parsed) -> Document {
    html::push_html(&mut parsed.document.html, parsed.events.into_iter());
    parsed.document
}
