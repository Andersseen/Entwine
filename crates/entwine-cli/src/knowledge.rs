//! Knowledge-convention presentation: coverage for `check`.
use entwine_core::{coverage, ArtifactKind, CoverageArea, KnowledgeRole};
use entwine_engine::Compilation;

fn heading(role: KnowledgeRole) -> &'static str {
    match role {
        KnowledgeRole::Project => "Entry point",
        KnowledgeRole::Architecture => "Architecture",
        KnowledgeRole::State => "Current state",
        KnowledgeRole::Roadmap => "Roadmap",
        KnowledgeRole::Decision => "Decisions",
        KnowledgeRole::Spec => "Specifications",
        KnowledgeRole::Other => "Other",
    }
}
fn missing_message(role: KnowledgeRole) -> &'static str {
    match role {
        KnowledgeRole::Project => "No project entry point found (recommended: docs/index.md)",
        KnowledgeRole::Architecture => {
            "No architecture document found (recommended: docs/architecture.md)"
        }
        KnowledgeRole::State => "No current-state document found (recommended: docs/state.md)",
        KnowledgeRole::Roadmap => "No roadmap document found (recommended: docs/roadmap.md)",
        KnowledgeRole::Decision => "No decision documents found (recommended: docs/decisions/)",
        KnowledgeRole::Spec => "No specification documents found (recommended: docs/specs/)",
        KnowledgeRole::Other => "",
    }
}

/// Print recommended-area presence and return the roles that are missing.
/// This is presence only; it says nothing about documentation quality.
pub(crate) fn report(compilation: &Compilation, strict: bool) -> Vec<KnowledgeRole> {
    let areas = coverage(
        &compilation
            .knowledge
            .documents
            .iter()
            .filter(|d| d.artifact == ArtifactKind::Documentation)
            .cloned()
            .collect::<Vec<_>>(),
    );
    eprintln!("\nProject knowledge");
    for area in &areas {
        print_area(area, strict);
    }
    areas
        .iter()
        .filter(|a| !a.is_present())
        .map(|a| a.role)
        .collect()
}

fn print_area(area: &CoverageArea, strict: bool) {
    let name = heading(area.role);
    if !area.is_present() {
        let mark = if strict { "✗" } else { "⚠" };
        eprintln!("\n{mark} {name}\n  {}", missing_message(area.role));
        return;
    }
    eprintln!("\n✓ {name}");
    let plural = matches!(area.role, KnowledgeRole::Decision | KnowledgeRole::Spec);
    if plural {
        eprintln!(
            "  {} document{}",
            area.documents.len(),
            if area.documents.len() == 1 { "" } else { "s" }
        );
    } else {
        for id in &area.documents {
            eprintln!("  docs/{}", id.0);
        }
    }
}

/// With discovery enabled, report what agent-facing knowledge was found and whether the
/// generated site publishes it. Silent when nothing is configured.
pub(crate) fn agent_summary(compilation: &Compilation) {
    let config = &compilation.config;
    if !config.discovers_agent_knowledge() {
        return;
    }
    let count = |kind: ArtifactKind| {
        compilation
            .knowledge
            .documents
            .iter()
            .filter(|d| d.artifact == kind)
            .count()
    };
    let scopes: std::collections::BTreeSet<_> = compilation
        .knowledge
        .documents
        .iter()
        .filter_map(|d| d.agent.as_ref().and_then(|a| a.scope.as_deref()))
        .collect();
    eprintln!(
        "✓ {} agent instruction file{} in {} scope{}\n✓ {} skill{}",
        count(ArtifactKind::AgentInstructions),
        if count(ArtifactKind::AgentInstructions) == 1 {
            ""
        } else {
            "s"
        },
        scopes.len(),
        if scopes.len() == 1 { "" } else { "s" },
        count(ArtifactKind::Skill),
        if count(ArtifactKind::Skill) == 1 {
            ""
        } else {
            "s"
        },
    );
    eprintln!(
        "  agent knowledge is {} in the generated site (site.include_agent_knowledge = {})",
        if config.site.include_agent_knowledge {
            "published"
        } else {
            "not published; context only"
        },
        config.site.include_agent_knowledge
    );
}
