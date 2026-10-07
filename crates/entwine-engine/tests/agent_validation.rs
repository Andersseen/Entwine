//! Regression fixtures distilled from real agent-heavy repositories: one canonical skill exposed
//! through several tool-specific manifests, nested instruction scopes, `@` imports, ignore
//! rules, and the publication boundary. Nothing here names a product; the patterns are generic.
use entwine_core::*;
use entwine_engine::*;
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}
fn config(publish: bool) -> Config {
    Config {
        discovery: DiscoveryConfig {
            agent_instructions: true,
            skills: true,
        },
        site: SiteConfig {
            include_agent_knowledge: publish,
        },
    }
}
fn compile_fixture(name: &str, publish: bool) -> Compilation {
    let root = fixture(name);
    let compilation = compile_with_config(&root, &root, None, &config(publish)).unwrap();
    assert!(!compilation.has_errors(), "{:?}", compilation.diagnostics);
    compilation
}
fn find<'a>(c: &'a Compilation, path: &str) -> &'a Document {
    c.knowledge
        .documents
        .iter()
        .find(|d| d.agent.as_ref().is_some_and(|a| a.path == path))
        .unwrap_or_else(|| panic!("{path} was not discovered"))
}
fn agent<'a>(c: &'a Compilation, path: &str) -> &'a AgentDetails {
    find(c, path).agent.as_ref().unwrap()
}
fn page(c: &Compilation, path: &str) -> String {
    let file = render(c)
        .into_iter()
        .find(|f| f.path == path)
        .unwrap_or_else(|| panic!("missing output {path}"));
    String::from_utf8(file.contents).unwrap()
}
fn write(root: &Path, name: &str, text: &str) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

#[test]
fn bridges_collapse_into_their_canonical_skill_only_on_provable_evidence() {
    let c = compile_fixture("agent-bridges", true);
    let canonical = |path: &str, exposures: &[&str]| {
        let a = agent(&c, path);
        assert!(a.exposure_of.is_none(), "{path} must stay canonical");
        let mut found: Vec<_> = a.exposures.iter().map(|id| id.0.clone()).collect();
        found.sort();
        let want: Vec<_> = exposures.iter().map(|p| format!("repo:{p}")).collect();
        assert_eq!(found, want, "exposures of {path}");
    };
    canonical(
        ".agents/skills/release/SKILL.md",
        &[".claude/skills/release/SKILL.md"],
    );
    canonical(
        ".agents/skills/triage/SKILL.md",
        &[
            ".claude/skills/triage/SKILL.md",
            ".codex/skills/triage/SKILL.md",
        ],
    );
    canonical(".agents/skills/lonely/SKILL.md", &[]);
    for bridge in [
        ".claude/skills/release/SKILL.md",
        ".claude/skills/triage/SKILL.md",
        ".codex/skills/triage/SKILL.md",
    ] {
        let a = agent(&c, bridge);
        assert_eq!(a.exposure_basis, Some(ExposureBasis::DeclaredLink));
        assert!(a.exposure_of.as_ref().unwrap().0.starts_with("repo:.agents/skills/"));
    }
    // Same name without a link, a link without the same name, and an unrelated skill that
    // merely shares a name with a canonical one all stay independent: no inference.
    for independent in [
        ".claude/skills/lookalike/SKILL.md",
        ".claude/skills/different/SKILL.md",
        "packages/pack/skills/triage/SKILL.md",
    ] {
        let a = agent(&c, independent);
        assert!(a.exposure_of.is_none() && a.exposures.is_empty(), "{independent}");
    }
    // Raw files versus logical skills.
    let skills: Vec<_> = c
        .knowledge
        .documents
        .iter()
        .filter(|d| d.artifact == ArtifactKind::Skill)
        .collect();
    let logical = skills
        .iter()
        .filter(|d| d.agent.as_ref().unwrap().exposure_of.is_none())
        .count();
    assert_eq!((skills.len(), logical), (9, 6));
    // The artifact kind never changes: an exposure is still a skill.
    assert!(skills.iter().all(|d| d.artifact == ArtifactKind::Skill));
    // The relationship stays in the model; the exposure is not hidden from context.
    assert!(c.knowledge.relations.contains(&Relation {
        source: DocumentId("repo:.claude/skills/release/SKILL.md".into()),
        target: DocumentId("repo:.agents/skills/release/SKILL.md".into()),
        kind: RelationKind::References,
    }));
    let json = context_json(&c.context).unwrap();
    assert!(json.contains("\"exposure_basis\": \"declared_link\""));
    assert_eq!(c.context.schema_version, "0.5");
}

#[test]
fn agents_views_present_logical_skills_with_their_exposures() {
    let c = compile_fixture("agent-bridges", true);
    let overview = page(&c, "__entwine/agents/index.html");
    // 6 logical skills, 3 exposures, 2 resources? only the release checklist.
    assert!(overview.contains("<dt>Skills</dt><dd>6</dd>"), "{overview}");
    assert!(overview.contains("<dt>Skill exposures</dt><dd>3</dd>"));
    assert!(overview.contains("<dt>Skill resources</dt><dd>1</dd>"));
    assert!(overview.contains("exposed through"));
    // Skills are grouped by the directory their folders live in.
    assert!(overview.contains("<code>.agents/skills</code>"));
    assert!(overview.contains("<code>packages/pack/skills</code>"));
    // Exposures are not listed as separate skills on the overview.
    assert_eq!(overview.matches("class=\"skill-row\"").count(), 6);
    // Scopes show a proper nesting without claiming provider precedence.
    let scopes = page(&c, "__entwine/agents/scopes/index.html");
    assert!(scopes.contains("Project root") && scopes.contains("packages/web"));
    assert!(scopes.contains("does not simulate"));
    // The canonical skill page answers where it lives and what exposes it.
    let canonical = page(&c, "__entwine/agents/skills/dot-agents/skills/release/index.html");
    for needle in [
        "<dt>Canonical source</dt>",
        "<dt>Exposed through</dt>",
        ".claude/skills/release/SKILL.md",
        "references/checklist.md",
        "Referenced by documentation",
        "Referenced by instructions",
    ] {
        assert!(canonical.contains(needle), "canonical page lacks {needle}");
    }
    // The exposure page points back, and says why Entwine believes it.
    let exposure = page(&c, "__entwine/agents/skills/dot-claude/skills/release/index.html");
    assert!(exposure.contains("only <strong>exposes</strong>"));
    assert!(exposure.contains("declares the same name"));
    // An independent skill that shares a name shows no exposure claim.
    let pack = page(&c, "__entwine/agents/skills/packages/pack/skills/triage/index.html");
    assert!(!pack.contains("exposes") && !pack.contains("Exposed through"));
}

#[test]
fn the_graph_collapses_exposures_but_keeps_every_relationship_and_node_in_text() {
    let c = compile_fixture("agent-bridges", true);
    let graph = page(&c, "__entwine/graph/index.html");
    // Exposure nodes exist but are hidden until asked for, with and without scripting.
    let exposure_nodes = graph.matches("is-exposure is-off").count();
    assert_eq!(exposure_nodes, 3);
    assert!(graph.contains("data-filter-exposures"));
    assert!(graph.contains("3 skill exposures collapsed into canonical skills"));
    assert!(graph.contains("Skill exposure (hidden by default)"));
    // The JSON payload says what each hidden node exposes.
    assert!(graph.contains("\"exposure_of\":\"repo:.agents/skills/triage/SKILL.md\""));
    // The static index is complete: every artifact is reachable, exposures nested under canonicals.
    assert_eq!(graph.matches("class=\"exposure-list\"").count(), 2);
    assert!(graph.contains("exposed through"));
    // Every relationship is listed, none dropped.
    assert_eq!(graph.matches(" references <a ").count(), c.graph.edges.len());
    // Colliding titles are told apart in labels and relationship lists.
    assert!(graph.contains("class=\"node-qualifier\""));
    // Graph model counts: raw nodes stay complete.
    assert_eq!(
        c.graph.nodes.iter().filter(|n| n.exposure_of.is_some()).count(),
        3
    );
    let canonical = c
        .graph
        .nodes
        .iter()
        .find(|n| n.id.0 == "repo:.agents/skills/triage/SKILL.md")
        .unwrap();
    assert_eq!(canonical.exposures, 2);
    assert_eq!(canonical.location.as_deref(), Some(".agents/skills"));
}

#[test]
fn instruction_imports_scopes_and_directory_links_behave_as_documented() {
    let c = compile_fixture("agent-bridges", false);
    // CLAUDE.md pulls in AGENTS.md with `@AGENTS.md`: a relationship, nothing more.
    let imports: Vec<_> = c
        .knowledge
        .relations
        .iter()
        .filter(|r| r.source.0 == "repo:CLAUDE.md")
        .map(|r| r.target.0.as_str())
        .collect();
    assert_eq!(imports, ["repo:AGENTS.md"]);
    // `@maintainers`, `@scope/package`, a code span, and a fenced block are not imports.
    let claude = find(&c, "CLAUDE.md");
    let destinations: Vec<_> = claude.links.iter().map(|l| l.destination.as_str()).collect();
    assert_eq!(destinations, ["@AGENTS.md"]);
    assert!(find(&c, "AGENTS.md")
        .links
        .iter()
        .all(|l| !l.destination.starts_with('@')));
    // Linking to a directory is normal for an agent file and raises no diagnostic.
    assert!(
        c.diagnostics
            .iter()
            .all(|d| !d.message.contains("does not exist") && !d.message.contains("not a file")),
        "{:?}",
        c.diagnostics
    );
    // Scope is structural: the nested file has its own directory scope.
    assert_eq!(agent(&c, "packages/web/AGENTS.md").scope.as_deref(), Some("packages/web"));
    assert_eq!(agent(&c, "CLAUDE.md").scope.as_deref(), Some(""));
    let _ = c
        .knowledge
        .documents
        .iter()
        .filter(|d| d.artifact == ArtifactKind::Documentation)
        .count();
}

#[test]
fn documentation_and_agent_files_cross_link_in_both_directions() {
    let c = compile_fixture("agent-bridges", true);
    let has = |source: &str, target: &str| {
        c.knowledge.relations.contains(&Relation {
            source: DocumentId(source.into()),
            target: DocumentId(target.into()),
            kind: RelationKind::References,
        })
    };
    assert!(has("index.md", "repo:AGENTS.md"));
    assert!(has("index.md", "repo:.agents/skills/release/SKILL.md"));
    assert!(has("repo:AGENTS.md", "architecture.md"));
    assert!(has("repo:.agents/skills/release/SKILL.md", "architecture.md"));
    let overview = page(&c, "__entwine/agents/index.html");
    assert!(overview.contains("Connected to documentation"));
    assert!(overview.contains("Documentation that references agent files"));
}

#[test]
fn gitignored_files_are_not_discovered_and_ignored_resources_are_not_listed() {
    let temp = tempfile::tempdir().unwrap();
    let r = temp.path();
    write(
        r,
        ".gitignore",
        "CLAUDE.local.md\n/private/\n*.scratch\nbuild-output/\n!private/keep/\n",
    );
    write(r, "docs/index.md", "# Ignore fixture\n");
    write(r, "AGENTS.md", "# Tracked\n");
    write(r, "CLAUDE.local.md", "# Never a convention anyway\n");
    write(r, "private/AGENTS.md", "# Ignored directory\n");
    write(r, "private/keep/AGENTS.md", "# Below an ignored parent\n");
    write(r, "build-output/AGENTS.md", "# Generated\n");
    write(r, "packages/web/AGENTS.md", "# Visible\n");
    write(r, "packages/web/.gitignore", "GEMINI.md\n");
    write(r, "packages/web/GEMINI.md", "# Ignored by the nested rule\n");
    write(r, "packages/api/GEMINI.md", "# Visible: the nested rule is scoped\n");
    write(
        r,
        ".claude/skills/shared/SKILL.md",
        "---\nname: shared\n---\n# Shared\n",
    );
    write(r, ".claude/skills/shared/references/notes.md", "# notes\n");
    write(r, ".claude/skills/shared/references/draft.scratch", "private\n");
    write(r, ".claude/skills/personal.scratch/SKILL.md", "---\nname: personal\n---\n");
    // Exclusions that live inside the repository but are never committed.
    write(r, ".git/info/exclude", "packages/api/GEMINI.md\n");
    let c = compile_with_config(r, r, None, &config(false)).unwrap();
    let paths: BTreeSet<_> = c
        .knowledge
        .documents
        .iter()
        .filter_map(|d| d.agent.as_ref().map(|a| a.path.as_str()))
        .collect();
    assert_eq!(
        paths,
        BTreeSet::from([
            "AGENTS.md",
            "packages/web/AGENTS.md",
            ".claude/skills/shared/SKILL.md"
        ])
    );
    assert_eq!(
        agent(&c, ".claude/skills/shared/SKILL.md").resources,
        ["references/notes.md"]
    );
    // Files seen and skipped: the nested-rule GEMINI.md and the `.git/info/exclude` one. Ignored
    // directories (`private/`, `build-output/`, `personal.scratch/`) are never entered at all.
    assert_eq!(c.ignored_agent_files, 2);
    // Git is not required: with no `.git`, the `.gitignore` files still apply.
    fs::remove_dir_all(r.join(".git")).unwrap();
    let without_git = compile_with_config(r, r, None, &config(false)).unwrap();
    assert!(without_git
        .knowledge
        .documents
        .iter()
        .all(|d| d.agent.as_ref().is_none_or(|a| !a.path.starts_with("private/"))));
    // docs/ is the documentation boundary and is never filtered by ignore rules.
    write(r, ".gitignore", "docs/\n*.md\n");
    let docs_only = compile_with_config(r, r, None, &Config::default()).unwrap();
    assert_eq!(docs_only.knowledge.documents.len(), 1);
}

#[test]
fn ignore_rules_in_a_monorepo_ancestor_apply_to_a_nested_project() {
    let temp = tempfile::tempdir().unwrap();
    let r = temp.path();
    write(r, ".gitignore", "secret/\n");
    write(r, "apps/site/docs/index.md", "# Site\n");
    write(r, "apps/site/AGENTS.md", "# Site\n");
    write(r, "apps/site/secret/AGENTS.md", "# Hidden by the repository root rule\n");
    write(r, "apps/site/.gitignore", "local/\n");
    write(r, "apps/site/local/AGENTS.md", "# Hidden by the project rule\n");
    let project = r.join("apps/site");
    let c = compile_with_config(&project, r, None, &config(false)).unwrap();
    let paths: Vec<_> = c
        .knowledge
        .documents
        .iter()
        .filter_map(|d| d.agent.as_ref().map(|a| a.path.as_str()))
        .collect();
    assert_eq!(paths, ["apps/site/AGENTS.md"]);
}

fn all_text(files: &[StaticFile]) -> Vec<(String, String)> {
    files
        .iter()
        .map(|f| {
            (
                f.path.clone(),
                String::from_utf8_lossy(&f.contents).into_owned(),
            )
        })
        .collect()
}

#[test]
fn private_agent_knowledge_is_absent_from_every_output_file_when_publication_is_off() {
    let c = compile_fixture("agent-private", false);
    // Discovery still happened: the context is complete.
    let context = context_json(&c.context).unwrap();
    for token in ["TOKEN-SKILL-NAME", "TOKEN-INSTRUCTION-BODY", "TOKEN-RESOURCE.md"] {
        assert!(context.contains(token), "context lacks {token}");
    }
    let files = render(&c);
    let paths: Vec<_> = files.iter().map(|f| f.path.as_str()).collect();
    assert!(paths.iter().all(|p| !p.contains("agents")), "{paths:?}");
    assert!(paths.iter().all(|p| !p.contains("TOKEN")), "{paths:?}");
    // Nothing a discovered file contains or is called appears in any page, script, or style.
    for (path, text) in all_text(&files) {
        for forbidden in [
            "TOKEN-",
            "secret-scope-dir",
            "Secret rules",
            "classified",
            "__entwine/agents",
            "agent-private",
        ] {
            assert!(!text.contains(forbidden), "{path} leaks {forbidden:?}");
        }
        // The shared script and stylesheet are static code; only pages carry per-project data.
        if path.ends_with(".html") {
            for marker in ["data-filter-exposures", "exposure_of", "is-exposure"] {
                assert!(!text.contains(marker), "{path} carries {marker:?}");
            }
        }
    }
    // The site model itself carries no agent entries.
    assert!(c.site.agents.is_none());
    assert!(c.graph.nodes.iter().all(|n| n.artifact == ArtifactKind::Documentation));
    assert!(c.graph.files.iter().all(|f| !f.path.contains("TOKEN")));
    // Control: publication makes the very same tokens appear, so the scan is meaningful.
    let published = compile_fixture("agent-private", true);
    let leaked: String = all_text(&render(&published))
        .into_iter()
        .map(|(_, text)| text)
        .collect();
    assert!(leaked.contains("TOKEN-SKILL-DESCRIPTION") && leaked.contains("TOKEN-RESOURCE.md"));
    assert!(leaked.contains("secret-scope-dir"));
}

#[test]
fn a_documentation_link_into_unpublished_agent_files_exposes_only_what_the_author_wrote() {
    let temp = tempfile::tempdir().unwrap();
    let r = temp.path();
    write(r, "docs/index.md", "# Home\n\nSee [rules](../AGENTS.md).\n");
    write(
        r,
        "AGENTS.md",
        "# Rules\n\nTOKEN-BODY and [skill](.agents/skills/s/SKILL.md).\n",
    );
    write(
        r,
        ".agents/skills/s/SKILL.md",
        "---\nname: TOKEN-NAME\ndescription: TOKEN-DESC\n---\n",
    );
    let c = compile_with_config(r, r, None, &config(false)).unwrap();
    let files = render(&c);
    for (path, text) in all_text(&files) {
        assert!(!text.contains("TOKEN-"), "{path} leaks content");
        assert!(!text.contains("__entwine/agents"), "{path} links agent pages");
    }
    // The author's own link is still shown as a labeled repository path, as without any config.
    assert!(page(&c, "index.html").contains("<code>AGENTS.md</code>"));
}

#[test]
fn zero_config_never_scans_or_surfaces_agent_files() {
    let root = fixture("agent-bridges");
    let c = compile(&root).unwrap();
    assert!(c.knowledge.documents.iter().all(|d| d.agent.is_none()));
    assert_eq!(c.ignored_agent_files, 0);
    assert!(c.site.agents.is_none());
    let files = render(&c);
    assert!(files.iter().all(|f| !f.path.contains("agents")));
    assert!(c.diagnostics.iter().all(|d| !d.source.starts_with("repo:")));
    // The graph for an ordinary docs repository carries none of the agent-only payload fields.
    let graph = page(&c, "__entwine/graph/index.html");
    for field in ["exposure_of", "exposed_through", "\"linked\"", "qualifier", "location"] {
        assert!(!graph.contains(field), "docs-only graph payload contains {field}");
    }
}

#[test]
fn the_graph_without_scripting_is_complete_and_the_payload_is_safe() {
    let c = compile_fixture("agent-bridges", true);
    let graph = page(&c, "__entwine/graph/index.html");
    // Static SVG, the legend, links on every node, and the complete text index need no script.
    assert!(graph.contains("<svg class=\"graph\""));
    assert!(graph.contains("class=\"graph-legend\""));
    assert!(graph.contains("<div class=\"graph-controls\" hidden>"));
    assert!(graph.contains("<aside class=\"graph-inspector\" aria-label=\"Selected node\" aria-live=\"polite\" hidden>"));
    assert_eq!(
        graph.matches("<a class=\"graph-node").count(),
        c.graph.nodes.len()
    );
    for node in &c.graph.nodes {
        assert!(graph.contains(&format!("data-node=\"{}\"", node.id.0)));
    }
    let data = graph
        .split("id=\"graph-data\">")
        .nth(1)
        .unwrap()
        .split("</script>")
        .next()
        .unwrap();
    assert!(!data.contains('<'));
}
