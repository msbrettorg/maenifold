use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PipelineDef {
    pub name: String,
    pub stages: Vec<StageDef>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StageDef {
    pub name: String,
    pub verdicts: Vec<String>,
    pub transitions: HashMap<String, String>,
}

pub fn load_pipeline(path: &Path) -> Result<PipelineDef> {
    load_pipeline_from_str(
        &fs::read_to_string(path)
            .with_context(|| format!("reading pipeline definition {}", path.display()))?,
    )
}

pub fn load_pipeline_from_str(json: &str) -> Result<PipelineDef> {
    let def: PipelineDef =
        serde_json::from_str(json).context("deserializing pipeline definition JSON")?;
    validate(&def)?;
    Ok(def)
}

pub fn load_pipeline_def(explicit_path: Option<&Path>, root: &Path) -> Result<PipelineDef> {
    if let Some(path) = explicit_path {
        load_pipeline(path)
    } else {
        let root_pipeline = root.join("pipeline.json");
        if root_pipeline.is_file() {
            load_pipeline(&root_pipeline)
        } else {
            load_pipeline_from_str(include_str!("../../../assets/pipelines/tdd.json"))
        }
    }
}

fn validate(def: &PipelineDef) -> Result<()> {
    if def.stages.is_empty() {
        bail!("pipeline must define at least one stage");
    }

    let mut names = HashSet::new();
    for stage in &def.stages {
        if !names.insert(stage.name.as_str()) {
            bail!("duplicate stage name '{}'", stage.name);
        }
    }

    for stage in &def.stages {
        for target in stage.transitions.values() {
            if target != "done" && target != "pending" && !names.contains(target.as_str()) {
                bail!(
                    "unknown transition target '{}' from stage '{}'",
                    target,
                    stage.name
                );
            }
        }
    }

    let stages = def
        .stages
        .iter()
        .map(|stage| (stage.name.as_str(), stage))
        .collect::<HashMap<_, _>>();
    if has_cycle(&stages) {
        bail!("pipeline must not contain cycles");
    }
    let start = def.stages[0].name.as_str();
    if !can_reach_done(start, &stages, &mut HashSet::new()) {
        bail!("pipeline must have a path from '{}' to 'done'", start);
    }
    Ok(())
}

fn has_cycle<'a>(stages: &HashMap<&'a str, &'a StageDef>) -> bool {
    let mut marks = HashMap::new();
    stages
        .keys()
        .copied()
        .any(|stage| visit(stage, stages, &mut marks))
}

fn visit<'a>(
    stage: &'a str,
    stages: &HashMap<&'a str, &'a StageDef>,
    marks: &mut HashMap<&'a str, u8>,
) -> bool {
    match marks.get(stage).copied().unwrap_or_default() {
        1 => return true,
        2 => return false,
        _ => {}
    }
    marks.insert(stage, 1);
    let cyclic = stages[stage]
        .transitions
        .values()
        .filter_map(|target| match target.as_str() {
            "done" | "pending" => None,
            next => Some(next),
        })
        .any(|next| visit(next, stages, marks));
    marks.insert(stage, 2);
    cyclic
}

fn can_reach_done<'a>(
    stage: &'a str,
    stages: &HashMap<&'a str, &'a StageDef>,
    seen: &mut HashSet<&'a str>,
) -> bool {
    seen.insert(stage)
        && stages[stage]
            .transitions
            .values()
            .any(|target| match target.as_str() {
                "done" => true,
                "pending" => false,
                next => can_reach_done(next, stages, seen),
            })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn asset(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/pipelines")
            .join(name)
    }

    #[test]
    fn load_valid_tdd_json() -> Result<()> {
        let def = load_pipeline(&asset("tdd.json"))?;
        assert_eq!(def.name, "tdd");
        assert_eq!(def.stages.len(), 3);
        Ok(())
    }

    #[test]
    fn load_valid_lightweight_json() -> Result<()> {
        let def = load_pipeline(&asset("lightweight.json"))?;
        assert_eq!(def.name, "lightweight");
        assert_eq!(def.stages.len(), 2);
        Ok(())
    }

    #[test]
    fn reject_duplicate_stage_names() {
        let err = load_pipeline_from_str(
            r#"{"name":"dup","stages":[
                {"name":"swe","verdicts":[],"transitions":{"pass":"done"}},
                {"name":"swe","verdicts":[],"transitions":{"pass":"done"}}
            ]}"#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("duplicate stage name 'swe'"));
    }

    #[test]
    fn reject_unknown_transition_target() {
        let err = load_pipeline_from_str(
            r#"{"name":"bad-target","stages":[
                {"name":"swe","verdicts":[],"transitions":{"pass":"nonexistent"}}
            ]}"#,
        )
        .unwrap_err();
        assert!(
            err.to_string()
                .contains("unknown transition target 'nonexistent'")
        );
    }

    #[test]
    fn reject_empty_stages() {
        let err = load_pipeline_from_str(r#"{"name":"empty","stages":[]}"#).unwrap_err();
        assert!(err.to_string().contains("at least one stage"));
    }

    #[test]
    fn reject_cycle_with_escape_path() {
        let err = load_pipeline_from_str(
            r#"{"name":"cyclic","stages":[{"name":"swe","verdicts":[],"transitions":{"pass":"blue","loop":"swe"}},{"name":"blue","verdicts":["pass"],"transitions":{"pass":"done"}}]}"#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("cycles"));
    }

    #[test]
    fn reject_no_path_to_done() {
        let err = load_pipeline_from_str(
            r#"{"name":"stuck","stages":[{"name":"swe","verdicts":[],"transitions":{"pass":"pending"}}]}"#,
        )
        .unwrap_err();
        assert!(err.to_string().contains("path from"));
    }

    #[test]
    fn load_from_str_works() -> Result<()> {
        let def = load_pipeline_from_str(
            r#"{"name":"inline","stages":[
                {"name":"swe","verdicts":[],"transitions":{"pass":"blue"}},
                {"name":"blue","verdicts":["pass","fail"],"transitions":{"pass":"done","fail":"pending"}}
            ]}"#,
        )?;
        assert_eq!(def.stages[1].name, "blue");
        assert_eq!(def.stages[1].verdicts, vec!["pass", "fail"]);
        Ok(())
    }
}
