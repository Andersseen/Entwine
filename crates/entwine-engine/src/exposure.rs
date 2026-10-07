//! Canonical skills and their exposures.
//!
//! Some repositories keep one provider-neutral skill and expose it to tools through thin
//! `SKILL.md` manifests that only point at it. Entwine must not present such a bridge as an
//! independent skill, but it also must not guess: identity is established only by evidence the
//! author declared or the file system proves. Today that evidence is a single rule:
//!
//! 1. the bridge declares a `name` in frontmatter and so does the target skill, and they match;
//! 2. the bridge links to the target's `SKILL.md`;
//! 3. the bridge carries no colocated files of its own.
//!
//! Similar descriptions, similar text, or equal folder names prove nothing. When the rule does
//! not hold, both manifests stay independent skills. Provider names play no part.
use entwine_core::*;
use std::collections::{BTreeMap, BTreeSet};

/// Upper bound on exposure chains; longer chains or cycles prove nothing and are ignored.
const MAX_CHAIN: usize = 8;

/// Record `exposure_of`, `exposure_basis`, and `exposures` on skill manifests. Idempotent.
pub(crate) fn detect(documents: &mut [Document]) {
    struct Skill {
        name: String,
        bare: bool,
        links: BTreeSet<DocumentId>,
    }
    let skills: BTreeMap<DocumentId, Skill> = documents
        .iter()
        .filter(|d| d.artifact == ArtifactKind::Skill)
        .filter_map(|d| {
            let agent = d.agent.as_ref()?;
            Some((
                d.id.clone(),
                Skill {
                    name: agent.name.clone()?,
                    bare: agent.resources.is_empty(),
                    links: d.links.iter().filter_map(|l| l.target.clone()).collect(),
                },
            ))
        })
        .collect();
    // A bridge points at exactly one same-named skill; anything ambiguous proves nothing.
    let mut direct: BTreeMap<&DocumentId, &DocumentId> = BTreeMap::new();
    for (id, skill) in &skills {
        if !skill.bare {
            continue;
        }
        let mut matches = skill
            .links
            .iter()
            .filter(|target| *target != id)
            .filter(|target| skills.get(*target).is_some_and(|t| t.name == skill.name));
        if let (Some(target), None) = (matches.next(), matches.next()) {
            direct.insert(id, target);
        }
    }
    let canonical = |start: &DocumentId| -> Option<DocumentId> {
        let mut seen = BTreeSet::from([start]);
        let mut current = start;
        for _ in 0..MAX_CHAIN {
            match direct.get(current) {
                Some(next) if seen.insert(next) => current = next,
                Some(_) => return None, // a cycle has no canonical end
                None => return (current != start).then(|| current.clone()),
            }
        }
        None
    };
    let resolved: BTreeMap<DocumentId, DocumentId> = direct
        .keys()
        .filter_map(|id| Some(((*id).clone(), canonical(id)?)))
        .collect();
    let mut exposures: BTreeMap<&DocumentId, Vec<DocumentId>> = BTreeMap::new();
    for (bridge, canonical) in &resolved {
        exposures.entry(canonical).or_default().push(bridge.clone());
    }
    for document in documents {
        let Some(agent) = document.agent.as_mut() else {
            continue;
        };
        agent.exposure_of = resolved.get(&document.id).cloned();
        agent.exposure_basis = agent
            .exposure_of
            .as_ref()
            .map(|_| ExposureBasis::DeclaredLink);
        agent.exposures = exposures.remove(&document.id).unwrap_or_default();
        agent.exposures.sort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown;

    fn skill(path: &str, name: Option<&str>, resources: &[&str], links: &[&str]) -> Document {
        let id = format!("{REPOSITORY_ID_PREFIX}{path}");
        let mut parsed = markdown::parse(
            &id,
            Route::from_source("x.md").unwrap(),
            "# Skill\n",
            &mut Vec::new(),
        );
        let doc = &mut parsed.document;
        doc.artifact = ArtifactKind::Skill;
        doc.links = links
            .iter()
            .map(|target| Link {
                destination: (*target).into(),
                line: 1,
                target: Some(DocumentId(format!("{REPOSITORY_ID_PREFIX}{target}"))),
                href: String::new(),
            })
            .collect();
        doc.agent = Some(AgentDetails {
            convention: AgentConvention::SkillMd,
            path: path.into(),
            scope: None,
            name: name.map(String::from),
            description: None,
            skill_directory: Some(crate::discovery::parent(path).into()),
            resources: resources.iter().map(|r| (*r).into()).collect(),
            exposure_of: None,
            exposure_basis: None,
            exposures: Vec::new(),
        });
        markdown::finish(parsed)
    }
    fn of<'a>(docs: &'a [Document], path: &str) -> &'a AgentDetails {
        docs.iter()
            .find_map(|d| d.agent.as_ref().filter(|a| a.path == path))
            .unwrap()
    }

    #[test]
    fn a_named_linking_bare_manifest_is_an_exposure_of_its_target() {
        let mut docs = vec![
            skill(".agents/skills/a/SKILL.md", Some("a"), &["notes.md"], &[]),
            skill(
                ".claude/skills/a/SKILL.md",
                Some("a"),
                &[],
                &[".agents/skills/a/SKILL.md"],
            ),
        ];
        detect(&mut docs);
        let canonical = of(&docs, ".agents/skills/a/SKILL.md");
        let bridge = of(&docs, ".claude/skills/a/SKILL.md");
        assert_eq!(
            bridge.exposure_of.as_ref().unwrap().0,
            "repo:.agents/skills/a/SKILL.md"
        );
        assert_eq!(bridge.exposure_basis, Some(ExposureBasis::DeclaredLink));
        assert_eq!(canonical.exposures.len(), 1);
        assert!(canonical.exposure_of.is_none() && bridge.exposures.is_empty());
    }

    #[test]
    fn anything_short_of_the_rule_keeps_skills_independent() {
        let target = ".agents/skills/a/SKILL.md";
        let cases = [
            // different declared name
            skill(".x/a/SKILL.md", Some("other"), &[], &[target]),
            // no declared name
            skill(".x/a/SKILL.md", None, &[], &[target]),
            // carries files of its own
            skill(".x/a/SKILL.md", Some("a"), &["extra.md"], &[target]),
            // same name, no link: a coincidence, not evidence
            skill(".x/a/SKILL.md", Some("a"), &[], &[]),
        ];
        for bridge in cases {
            let mut docs = vec![skill(target, Some("a"), &[], &[]), bridge];
            detect(&mut docs);
            assert!(docs.iter().all(|d| {
                let a = d.agent.as_ref().unwrap();
                a.exposure_of.is_none() && a.exposures.is_empty()
            }));
        }
    }

    #[test]
    fn chains_resolve_to_the_end_and_cycles_prove_nothing() {
        let mut chain = vec![
            skill("c/SKILL.md", Some("n"), &["r.md"], &[]),
            skill("b/SKILL.md", Some("n"), &[], &["c/SKILL.md"]),
            skill("a/SKILL.md", Some("n"), &[], &["b/SKILL.md"]),
        ];
        detect(&mut chain);
        assert_eq!(
            of(&chain, "a/SKILL.md").exposure_of.as_ref().unwrap().0,
            "repo:c/SKILL.md"
        );
        assert_eq!(of(&chain, "c/SKILL.md").exposures.len(), 2);
        let mut cycle = vec![
            skill("a/SKILL.md", Some("n"), &[], &["b/SKILL.md"]),
            skill("b/SKILL.md", Some("n"), &[], &["a/SKILL.md"]),
        ];
        detect(&mut cycle);
        assert!(cycle
            .iter()
            .all(|d| d.agent.as_ref().unwrap().exposure_of.is_none()));
    }

    #[test]
    fn linking_several_same_named_skills_is_ambiguous() {
        let mut docs = vec![
            skill("x/SKILL.md", Some("n"), &["r"], &[]),
            skill("y/SKILL.md", Some("n"), &["r"], &[]),
            skill("z/SKILL.md", Some("n"), &[], &["x/SKILL.md", "y/SKILL.md"]),
        ];
        detect(&mut docs);
        assert!(docs
            .iter()
            .all(|d| d.agent.as_ref().unwrap().exposure_of.is_none()));
    }
}
