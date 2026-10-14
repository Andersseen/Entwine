//! Render a project with both page renderers and compare the output semantically.
//! The project is only read; both trees are written below `<out>` for inspection.
//! Run with `cargo run -p entwine-engine --features flowview-renderer --example
//! renderer_parity -- <out> <project>...`.
//!
//! Whitespace between tags and the `/>` spelling of void elements are not
//! semantic, so both are normalized before comparison. Everything else, including
//! text, attributes, element order, hrefs and aria-labels, must match exactly.

use entwine_core::RendererKind;
use entwine_engine::{compile_configured, render_selected, StaticFile};
use std::{fs, path::Path, process::ExitCode};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let out = args
        .next()
        .expect("usage: renderer_parity <out> <project>...");
    let mut failures = 0;
    for project in args {
        let project = fs::canonicalize(&project).expect("project exists");
        let name = project.file_name().unwrap().to_string_lossy().into_owned();
        let mut compilation = compile_configured(&project).expect("project compiles");
        assert!(!compilation.has_errors(), "{name} has fatal diagnostics");
        compilation.config.site.renderer = RendererKind::Builtin;
        let builtin = render_selected(&compilation).expect("builtin renders");
        compilation.config.site.renderer = RendererKind::Flowview;
        let flowview = render_selected(&compilation).expect("flowview renders");
        write_tree(&Path::new(&out).join(&name).join("builtin"), &builtin);
        write_tree(&Path::new(&out).join(&name).join("flowview"), &flowview);

        let paths = |files: &[StaticFile]| files.iter().map(|f| f.path.clone()).collect::<Vec<_>>();
        if paths(&builtin) != paths(&flowview) {
            println!("{name}: generated file lists differ");
            failures += 1;
        }
        let (mut html, mut equal, mut hrefs, mut labels) = (0, 0, 0, 0);
        for (expected, actual) in builtin.iter().zip(&flowview) {
            if !expected.path.ends_with(".html") {
                if expected.contents != actual.contents {
                    println!("{name}: {} differs", expected.path);
                    failures += 1;
                }
                continue;
            }
            html += 1;
            let expected = normalize(&String::from_utf8_lossy(&expected.contents));
            let actual_text = normalize(&String::from_utf8_lossy(&actual.contents));
            hrefs += attribute_values(&expected, "href").len();
            labels += attribute_values(&expected, "aria-label").len();
            if expected == actual_text {
                equal += 1;
            } else {
                let at = expected
                    .bytes()
                    .zip(actual_text.bytes())
                    .position(|(a, b)| a != b)
                    .unwrap_or(expected.len().min(actual_text.len()));
                let window =
                    |text: &str| text[at.saturating_sub(80)..(at + 80).min(text.len())].to_owned();
                println!(
                    "{name}: {} differs\n  builtin:  {}\n  flowview: {}",
                    actual.path,
                    window(&expected),
                    window(&actual_text)
                );
                failures += 1;
            }
        }
        println!(
            "{name}: {} files, {html} HTML, {equal} semantically equal, {hrefs} hrefs, {labels} aria-labels",
            builtin.len()
        );
    }
    if failures == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn write_tree(root: &Path, files: &[StaticFile]) {
    let _ = fs::remove_dir_all(root);
    for file in files {
        let path = root.join(&file.path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, &file.contents).unwrap();
    }
}

/// Drop whitespace between tags, collapse other whitespace runs, and spell void
/// elements without a trailing slash.
fn normalize(html: &str) -> String {
    let collapsed = html.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed
        .replace(" />", ">")
        .replace("/>", ">")
        .replace("> <", "><")
        .replace("> ", ">")
        .replace(" <", "<")
}

fn attribute_values(html: &str, name: &str) -> Vec<String> {
    let marker = format!(" {name}=\"");
    html.split(&marker)
        .skip(1)
        .filter_map(|rest| rest.split_once('"').map(|(value, _)| value.to_owned()))
        .collect()
}
