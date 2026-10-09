//! Optional configuration, agent-facing artifact discovery, scopes, and publication policy.
use entwine_core::*;
use entwine_engine::*;
use std::{fs, path::Path};

fn write(root: &Path, name: &str, text: &str) {
    let path = root.join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// A small repository with every supported convention at several depths.
fn repository() -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    let r = temp.path();
    write(
        r,
        "docs/index.md",
        "# Home\n\nSee [architecture](architecture.md) and the [agent rules](../AGENTS.md).\n",
    );
    write(
        r,
        "docs/architecture.md",
        "---\ntype: architecture\n---\n# Architecture\n\nBack to [home](index.md).\n",
    );
    write(r, "AGENTS.md", "# Rules\n\nRead [the architecture](docs/architecture.md). Web rules: [web](packages/web/AGENTS.md).\n");
    write(r, "CLAUDE.md", "See [AGENTS.md](AGENTS.md).\n");
    write(
        r,
        "packages/web/AGENTS.md",
        "# Web\n\nExtends [root](../../AGENTS.md).\n",
    );
    write(r, "packages/api/GEMINI.md", "# API\n\nNo links.\n");
    write(
        r,
        ".claude/skills/triage/SKILL.md",
        "---\nname: triage\ndescription: >\n  Sort reports\n  quickly.\n---\n# Triage\n\nUse [the checklist](references/checklist.md) and the [docs](../../../docs/architecture.md).\n",
    );
    write(
        r,
        ".claude/skills/triage/references/checklist.md",
        "# Checklist\n",
    );
    write(
        r,
        ".claude/skills/triage/scripts/run.sh",
        "echo never executed\n",
    );
    // Must never be discovered.
    write(r, "node_modules/pkg/AGENTS.md", "# vendored\n");
    write(r, "docs/AGENTS.md", "# inside docs\n");
    temp
}
fn config(instructions: bool, skills: bool, publish: bool) -> Config {
    Config {
        discovery: DiscoveryConfig {
            agent_instructions: instructions,
            skills,
        },
        site: SiteConfig {
            include_agent_knowledge: publish,
            ..Default::default()
        },
    }
}
fn compile_with(root: &Path, config: &Config) -> Compilation {
    let c = compile_with_config(root, root, None, config).unwrap();
    assert!(!c.has_errors(), "{:?}", c.diagnostics);
    c
}
fn paths(c: &Compilation, kind: ArtifactKind) -> Vec<String> {
    c.knowledge
        .documents
        .iter()
        .filter(|d| d.artifact == kind)
        .map(|d| d.agent.as_ref().unwrap().path.clone())
        .collect()
}
fn page(c: &Compilation, path: &str) -> Option<String> {
    render(c)
        .unwrap()
        .into_iter()
        .find(|f| f.path == path)
        .map(|f| String::from_utf8(f.contents).unwrap())
}

#[test]
fn config_is_optional_small_and_strict() {
    let temp = tempfile::tempdir().unwrap();
    assert_eq!(load_config(temp.path()).unwrap(), Config::default());
    assert!(!Config::default().discovers_agent_knowledge());
    write(
        temp.path(),
        "entwine.toml",
        "[discovery]\nskills = true\n\n[site]\ninclude_agent_knowledge = true\n",
    );
    let loaded = load_config(temp.path()).unwrap();
    assert!(loaded.discovery.skills && !loaded.discovery.agent_instructions);
    assert!(loaded.site.include_agent_knowledge);
    for bad in [
        "[discovery]\nskils = true\n",
        "[unknown]\nx = 1\n",
        "[site]\ninclude_agent_knowledge = \"yes\"\n",
        "not toml ===",
    ] {
        let error = parse_config(bad).unwrap_err().to_string();
        assert!(error.starts_with("Invalid entwine.toml:"), "{error}");
    }
}

#[test]
fn zero_config_ignores_agent_files_and_keeps_references_plain() {
    let temp = repository();
    let c = compile_with(temp.path(), &Config::default());
    assert_eq!(c.knowledge.documents.len(), 3); // docs/index, architecture, docs/AGENTS
    assert!(c
        .knowledge
        .documents
        .iter()
        .all(|d| d.artifact == ArtifactKind::Documentation));
    // A link to AGENTS.md is an ordinary repository reference, exactly as before.
    let index = c
        .knowledge
        .documents
        .iter()
        .find(|d| d.id.0 == "index.md")
        .unwrap();
    assert_eq!(index.repository_references.len(), 1);
    assert_eq!(index.repository_references[0].path, "AGENTS.md");
    assert!(c.site.agents.is_none());
    assert!(page(&c, "__entwine/agents/index.html").is_none());
    assert!(c.context.documents.iter().all(|d| d.agent.is_none()));
}

#[test]
fn every_convention_is_discovered_with_structure_and_nothing_vendored() {
    let temp = repository();
    let c = compile_with(temp.path(), &config(true, true, false));
    assert_eq!(
        paths(&c, ArtifactKind::AgentInstructions),
        [
            "AGENTS.md",
            "CLAUDE.md",
            "packages/api/GEMINI.md",
            "packages/web/AGENTS.md"
        ]
    );
    assert_eq!(
        paths(&c, ArtifactKind::Skill),
        [".claude/skills/triage/SKILL.md"]
    );
    let find = |path: &str| {
        c.knowledge
            .documents
            .iter()
            .find(|d| d.agent.as_ref().is_some_and(|a| a.path == path))
            .unwrap()
    };
    // Hierarchical scope, convention, and project-relative titles.
    let web = find("packages/web/AGENTS.md");
    assert_eq!(web.id.0, "repo:packages/web/AGENTS.md");
    assert_eq!(
        web.agent.as_ref().unwrap().scope.as_deref(),
        Some("packages/web")
    );
    assert_eq!(
        web.agent.as_ref().unwrap().convention,
        AgentConvention::AgentsMd
    );
    assert_eq!(web.title, "packages/web/AGENTS.md");
    assert_eq!(
        find("AGENTS.md").agent.as_ref().unwrap().scope.as_deref(),
        Some("")
    );
    assert_eq!(
        find("packages/api/GEMINI.md")
            .agent
            .as_ref()
            .unwrap()
            .convention,
        AgentConvention::GeminiMd
    );
    assert_eq!(
        find("CLAUDE.md").agent.as_ref().unwrap().convention,
        AgentConvention::ClaudeMd
    );
    // The role stays Other: artifact kind is a separate axis.
    assert!(c
        .knowledge
        .documents
        .iter()
        .filter(|d| d.artifact.is_agent_facing())
        .all(|d| d.role == KnowledgeRole::Other));
    // Skills: frontmatter, folder identity, colocated resources (never read or run).
    let skill = find(".claude/skills/triage/SKILL.md");
    let details = skill.agent.as_ref().unwrap();
    assert_eq!(skill.title, "triage");
    assert_eq!(
        details.description.as_deref(),
        Some("Sort reports quickly.")
    );
    assert_eq!(
        details.skill_directory.as_deref(),
        Some(".claude/skills/triage")
    );
    assert_eq!(
        details.resources,
        ["references/checklist.md", "scripts/run.sh"]
    );
    assert_eq!(details.scope, None);
    // The skill's link to a colocated file is a repository reference; its docs link is a relation.
    assert_eq!(skill.repository_references.len(), 1);
    assert!(c.knowledge.relations.contains(&Relation {
        source: skill.id.clone(),
        target: DocumentId("architecture.md".into()),
        kind: RelationKind::References,
    }));
}

#[test]
fn categories_can_be_enabled_independently() {
    let temp = repository();
    let only_skills = compile_with(temp.path(), &config(false, true, false));
    assert!(paths(&only_skills, ArtifactKind::AgentInstructions).is_empty());
    assert_eq!(paths(&only_skills, ArtifactKind::Skill).len(), 1);
    let only_instructions = compile_with(temp.path(), &config(true, false, false));
    assert!(paths(&only_instructions, ArtifactKind::Skill).is_empty());
    assert_eq!(
        paths(&only_instructions, ArtifactKind::AgentInstructions).len(),
        4
    );
}

#[test]
fn discovery_without_publication_keeps_agents_out_of_the_site_but_in_context() {
    let temp = repository();
    let c = compile_with(temp.path(), &config(true, true, false));
    // Context is complete and carries the new schema fields.
    assert_eq!(c.context.schema_version, "0.4");
    assert_eq!(
        c.context
            .documents
            .iter()
            .filter(|d| d.agent.is_some())
            .count(),
        5
    );
    let json = context_json(&c.context).unwrap();
    assert!(
        json.contains("\"artifact\": \"agent_instructions\"")
            && json.contains("\"artifact\": \"skill\"")
    );
    assert!(json.contains("\"artifact\": \"documentation\""));
    let markdown = context_markdown(&c.context);
    assert!(markdown.contains("[agent_instructions]") && markdown.contains("Scope: packages/web"));
    // The doc → AGENTS.md link is a real relationship in the model and in context...
    let relation = Relation {
        source: DocumentId("index.md".into()),
        target: DocumentId("repo:AGENTS.md".into()),
        kind: RelationKind::References,
    };
    assert!(c.knowledge.relations.contains(&relation));
    assert!(c.context.relationships.contains(&relation));
    // ...but nothing agent-facing reaches the public site, graph, or any output file.
    assert!(c.site.agents.is_none());
    assert!(c
        .graph
        .nodes
        .iter()
        .all(|n| n.artifact == ArtifactKind::Documentation));
    assert!(c
        .graph
        .edges
        .iter()
        .all(|e| !e.source.0.starts_with("repo:") && !e.target.0.starts_with("repo:")));
    let files = render(&c).unwrap();
    assert!(files
        .iter()
        .all(|f| !f.path.starts_with("__entwine/agents")));
    for file in &files {
        if let Ok(text) = String::from_utf8(file.contents.clone()) {
            assert!(
                !text.contains("packages/web/AGENTS.md"),
                "{} leaks an agent path",
                file.path
            );
            assert!(!text.contains("triage"), "{} leaks a skill", file.path);
        }
    }
    // The document still renders its link as a labeled repository path.
    let home = page(&c, "index.html").unwrap();
    assert!(home.contains("repository-reference") && home.contains("<code>AGENTS.md</code>"));
    assert!(!home.contains("Agents</a>"));
}

#[test]
fn explicit_publication_adds_agent_pages_graph_nodes_and_links() {
    let temp = repository();
    let c = compile_with(temp.path(), &config(true, true, true));
    let agents = c.site.agents.as_ref().unwrap();
    assert_eq!(agents.entries.len(), 5);
    for path in [
        "__entwine/agents/index.html",
        "__entwine/agents/instructions/index.html",
        "__entwine/agents/skills/index.html",
        "__entwine/agents/scopes/index.html",
        "__entwine/agents/instructions/AGENTS/index.html",
        "__entwine/agents/instructions/packages/web/AGENTS/index.html",
        "__entwine/agents/skills/dot-claude/skills/triage/index.html",
    ] {
        assert!(page(&c, path).is_some(), "missing {path}");
    }
    let skill = page(
        &c,
        "__entwine/agents/skills/dot-claude/skills/triage/index.html",
    )
    .unwrap();
    assert!(skill.contains("Sort reports quickly."));
    assert!(skill.contains("references/checklist.md") && skill.contains("scripts/run.sh"));
    assert!(skill.contains("../../../../../../architecture/")); // link to documentation
    let scopes = page(&c, "__entwine/agents/scopes/index.html").unwrap();
    assert!(scopes.contains("Project root") && scopes.contains("packages/web"));
    // Documentation links to the artifact page and is no longer a repository reference.
    let home = page(&c, "index.html").unwrap();
    assert!(
        home.contains("href=\"__entwine/agents/instructions/AGENTS/\"")
            || home.contains("__entwine/agents/instructions/AGENTS/")
    );
    assert!(home.contains(">Agents</a>"));
    let graph = String::from_utf8(render_graph(&c.graph).contents).unwrap();
    for needle in [
        "data-kind=\"agent_instructions\"",
        "data-kind=\"skill\"",
        "data-filter-kind=\"agent_instructions\"",
        "data-filter-kind=\"skill\"",
        "data-filter-kind=\"documentation\"",
    ] {
        assert!(graph.contains(needle), "graph lacks {needle}");
    }
    // The text relationship list stays complete: every edge is listed.
    assert_eq!(
        graph.matches(" references <a ").count(),
        c.graph.edges.len()
    );
    let knowledge = page(&c, "__entwine/knowledge/index.html").unwrap();
    assert!(knowledge.contains("Agent knowledge"));
}

#[test]
fn graph_output_is_stable_and_ships_its_script() {
    let temp = repository();
    let c = compile_with(temp.path(), &config(true, true, true));
    let first = render(&c).unwrap();
    let second = render(&compile_with(temp.path(), &config(true, true, true))).unwrap();
    assert_eq!(first, second);
    let script = first
        .iter()
        .find(|f| f.path == "__entwine/graph.js")
        .unwrap();
    assert!(script.contents.len() < 40_000);
    let graph = String::from_utf8(render_graph(&c.graph).contents).unwrap();
    assert!(graph.contains("<script defer src=\"../graph.js\">"));
    // The embedded data cannot terminate its script element.
    let data = graph
        .split("id=\"graph-data\">")
        .nth(1)
        .unwrap()
        .split("</script>")
        .next()
        .unwrap();
    assert!(!data.contains('<'));
    assert!(graph.contains("Select a node to see its details"));
    assert!(graph.contains("<div class=\"graph-controls\" hidden>"));
}

#[test]
fn discovery_never_follows_symlinks_and_scans_only_the_project() {
    let temp = repository();
    let outside = tempfile::tempdir().unwrap();
    write(outside.path(), "AGENTS.md", "# outside\n");
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(outside.path(), temp.path().join("linked")).unwrap();
        std::os::unix::fs::symlink(
            outside.path().join("AGENTS.md"),
            temp.path().join("LINK-AGENTS.md"),
        )
        .unwrap();
    }
    let c = compile_with(temp.path(), &config(true, true, false));
    assert!(paths(&c, ArtifactKind::AgentInstructions)
        .iter()
        .all(|p| !p.contains("linked")));
    assert_eq!(paths(&c, ArtifactKind::AgentInstructions).len(), 4);
}

#[test]
fn monorepo_project_titles_and_scopes_are_project_relative() {
    let temp = tempfile::tempdir().unwrap();
    let r = temp.path();
    write(r, "apps/site/docs/index.md", "# Site\n");
    write(r, "apps/site/AGENTS.md", "# Site rules\n");
    write(r, "apps/site/lib/AGENTS.md", "# Lib rules\n");
    write(r, "apps/other/AGENTS.md", "# Not ours\n");
    let project = r.join("apps/site");
    let c = compile_with_config(&project, r, None, &config(true, true, false)).unwrap();
    assert_eq!(
        paths(&c, ArtifactKind::AgentInstructions),
        ["apps/site/AGENTS.md", "apps/site/lib/AGENTS.md"]
    );
    let scopes: Vec<_> = c
        .knowledge
        .documents
        .iter()
        .filter_map(|d| d.agent.as_ref()?.scope.clone())
        .collect();
    assert_eq!(scopes, ["", "lib"]);
}

#[test]
fn malformed_agent_files_warn_instead_of_failing_the_build() {
    let temp = repository();
    write(
        temp.path(),
        "packages/web/AGENTS.md",
        "---\ntitle: [unclosed\n---\n# Web\n\n[missing](nope.md) [bad](javascript:alert(1))\n",
    );
    let c = compile_with(temp.path(), &config(true, true, false));
    let warnings: Vec<_> = c
        .diagnostics
        .iter()
        .filter(|d| d.source.starts_with("repo:packages/web"))
        .collect();
    assert!(warnings.len() >= 2);
    assert!(warnings
        .iter()
        .all(|d| d.severity == DiagnosticSeverity::Warning));
}
