//! Deliberate complete projection snapshots; only the package version is normalized.
use std::{fs, path::Path};
#[test]
fn static_projections_and_context_match_reviewed_goldens() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let c = entwine_engine::compile(&root.join("fixtures/projection-golden")).unwrap();
    assert!(!c.has_errors(), "{:?}", c.diagnostics);
    let mut outputs = entwine_engine::render(&c)
        .unwrap()
        .into_iter()
        .filter(|f| {
            matches!(
                f.path.as_str(),
                "index.html"
                    | "specs/index.html"
                    | "__entwine/graph/index.html"
                    | "__entwine/knowledge/index.html"
            )
        })
        .map(|f| {
            (
                f.path.replace('/', "__"),
                String::from_utf8(f.contents).unwrap(),
            )
        })
        .collect::<Vec<_>>();
    outputs.push((
        "context.json".into(),
        entwine_engine::context_json(&c.context).unwrap(),
    ));
    for (name, output) in outputs {
        let output = output.replace(env!("CARGO_PKG_VERSION"), "PACKAGE_VERSION");
        let path = root.join("golden").join(name);
        if std::env::var_os("ENTWINE_UPDATE_GOLDENS").is_some() {
            fs::write(&path, &output).unwrap();
        }
        assert_eq!(
            output,
            fs::read_to_string(&path).unwrap(),
            "{}",
            path.display()
        );
    }
}
