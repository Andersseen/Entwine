//! Helpers shared by the page-renderer comparison tests.
#![allow(dead_code)]

/// Drop whitespace between tags, collapse other whitespace runs, and spell void
/// elements without a trailing slash. None of these change the parsed document.
pub fn normalize(html: &str) -> String {
    html.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(" />", ">")
        .replace("/>", ">")
        .replace("> <", "><")
        .replace("> ", ">")
        .replace(" <", "<")
}

pub fn attribute_values(html: &str, name: &str) -> Vec<String> {
    html.split(&format!(" {name}=\""))
        .skip(1)
        .filter_map(|rest| rest.split_once('"').map(|(value, _)| value.to_owned()))
        .collect()
}
