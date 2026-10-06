use crate::{checked, output, provider};
use entwine_engine::Compilation;
use notify::{RecursiveMode, Watcher};
use percent_encoding::percent_decode_str;
use std::collections::BTreeSet;
use std::{
    fs, io,
    path::Path,
    sync::mpsc,
    time::{Duration, Instant},
};
use tiny_http::{Header, Method, Response, Server, StatusCode};

fn header(name: &str, value: &str) -> Result<Header, io::Error> {
    Header::from_bytes(name, value).map_err(|_| io::Error::other("Invalid HTTP header"))
}
fn serve(request: tiny_http::Request, root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if request.method() != &Method::Get && request.method() != &Method::Head {
        request.respond(Response::empty(StatusCode(405)))?;
        return Ok(());
    }
    let url = request.url().split('?').next().unwrap_or("/");
    let decoded = match percent_decode_str(url).decode_utf8() {
        Ok(value) => value,
        Err(_) => {
            request.respond(Response::empty(StatusCode(400)))?;
            return Ok(());
        }
    };
    if decoded.split('/').any(|s| s == ".." || s.starts_with('.'))
        || decoded.contains('\\')
        || decoded.contains('\0')
    {
        request.respond(Response::empty(StatusCode(403)))?;
        return Ok(());
    }
    let mut path = root.join(decoded.trim_start_matches('/'));
    if path.is_dir() {
        if !url.ends_with('/') {
            let response = Response::empty(StatusCode(308))
                .with_header(header("Location", &format!("{url}/"))?);
            request.respond(response)?;
            return Ok(());
        }
        path = path.join("index.html");
    }
    let path = match path.canonicalize() {
        Ok(path) if path.starts_with(root) => path,
        _ => {
            request.respond(Response::empty(StatusCode(404)))?;
            return Ok(());
        }
    };
    let content_type = match path.extension().and_then(|s| s.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("json") => "application/json",
        _ => "application/octet-stream",
    };
    let bytes = fs::read(path)?;
    let length = bytes.len();
    let body = if request.method() == &Method::Head {
        Vec::new()
    } else {
        bytes
    };
    let response = Response::new(
        StatusCode(200),
        vec![
            header("Content-Type", content_type)?,
            header("Cache-Control", "no-store")?,
            header("X-Content-Type-Options", "nosniff")?,
        ],
        io::Cursor::new(body),
        Some(length),
        None,
    );
    request.respond(response)?;
    Ok(())
}

pub(crate) fn dev(project: &Path, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let server = Server::http(("127.0.0.1", port)).map_err(|e| {
        io::Error::other(format!("Cannot bind localhost:{port}: {e}. Choose --port."))
    })?;
    let (sender, receiver) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        let _ = sender.send(event);
    })?;
    watcher.watch(&project.join("docs"), RecursiveMode::Recursive)?;
    let initial = checked(project)?;
    output::publish(project, &initial)?;
    let repository = provider::inspect(project);
    let repository_root = repository.root.as_deref().unwrap_or(project);
    let mut watched = BTreeSet::new();
    watch_references(
        &mut watcher,
        &initial,
        repository_root,
        project,
        &mut watched,
    )?;
    let root = project.join("dist").canonicalize()?;
    eprintln!("Entwine dev server\n\nLocal: http://localhost:{port}\nWatching docs/ (refresh your browser after changes)");
    let mut changed = None;
    let mut changed_paths = BTreeSet::new();
    loop {
        while let Ok(event) = receiver.try_recv() {
            match event {
                Ok(event) if !matches!(event.kind, notify::EventKind::Access(_)) => {
                    for path in event.paths {
                        let relative = path.strip_prefix(project).unwrap_or(&path);
                        if relative.components().next().is_some_and(|c| {
                            let name = c.as_os_str().to_string_lossy();
                            name == "dist" || name == ".git" || name.starts_with(".entwine-")
                        }) {
                            continue;
                        }
                        changed_paths.insert(relative.display().to_string());
                        changed = Some(Instant::now());
                    }
                }
                Err(error) => eprintln!("warning: file watcher: {error}"),
                _ => {}
            }
        }
        if changed.is_some_and(|time| time.elapsed() >= Duration::from_millis(200)) {
            changed = None;
            eprintln!(
                "Changed: {}",
                changed_paths
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            changed_paths.clear();
            match checked(project).and_then(|compilation| {
                output::publish(project, &compilation)?;
                watch_references(
                    &mut watcher,
                    &compilation,
                    repository_root,
                    project,
                    &mut watched,
                )?;
                Ok(())
            }) {
                Ok(()) => eprintln!(
                    "✓ rebuilt documentation · http://localhost:{port} · refresh to read changes"
                ),
                Err(error) => eprintln!("error: {error}; serving the last successful build"),
            }
        }
        if let Some(request) = server.recv_timeout(Duration::from_millis(100))? {
            if let Err(error) = serve(request, &root) {
                eprintln!("warning: HTTP request: {error}");
            }
        }
    }
}

fn watch_references(
    watcher: &mut impl Watcher,
    compilation: &Compilation,
    repository: &Path,
    project: &Path,
    watched: &mut BTreeSet<std::path::PathBuf>,
) -> Result<(), notify::Error> {
    for reference in compilation
        .knowledge
        .documents
        .iter()
        .flat_map(|d| &d.repository_references)
    {
        if let Some(parent) = repository.join(&reference.path).parent() {
            if !parent.starts_with(project.join("docs")) && watched.insert(parent.to_path_buf()) {
                watcher.watch(parent, RecursiveMode::NonRecursive)?;
            }
        }
    }
    Ok(())
}
