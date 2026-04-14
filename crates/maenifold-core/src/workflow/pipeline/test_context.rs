use crate::config::Config;
use crate::test_util::ConfigResetGuard;
use crate::tools::ToolContext;
use crate::util::json::OutputFormat;
use serde_json::json;
use std::fs;
use tempfile::TempDir;

pub(super) fn pipeline_test_context(
    output_format: OutputFormat,
) -> (TempDir, ToolContext, ConfigResetGuard) {
    let tmp = TempDir::new().expect("temp dir");
    Config::override_root(tmp.path());
    Config::set_database_path(tmp.path().join("memory.db"));
    let config = Config::current();
    config.ensure_directories().expect("ensure directories");
    fs::create_dir_all(config.assets_path.join("workflows")).expect("create workflows dir");
    fs::create_dir_all(config.assets_path.join("usage/tools")).expect("create usage dir");
    fs::write(
        config.assets_path.join("usage/tools/pipeline.md"),
        "# Pipeline\n\nCreates and manages pipeline sessions composed of workflow nodes.\n\n## Operations\n\n- `definition`: create a pipeline session from `nodes` and `edges`.\n- `run`: start autonomous execution for a pipeline session.\n- `cancel`: cancel a running pipeline.\n- `view`: inspect the current pipeline state.\n- `dispatch`: claim ready nodes for execution.\n- `restart`: recreate a node session for an abandoned or stuck node.\n- `rewind`: reset selected node IDs and their downstream nodes.\n- `delete`: permanently delete the pipeline session and associated workflow sessions.\n- `graft`: add nodes and edges to an existing pipeline.\n- `join`: merge another pipeline into this pipeline.\n- `fork`: clone a subgraph into a new pipeline session.\n",
    )
    .expect("write help");

    for workflow_id in ["test-workflow-a", "test-workflow-b"] {
        fs::write(
            config
                .assets_path
                .join("workflows")
                .join(format!("{workflow_id}.json")),
            serde_json::to_string_pretty(&json!({
                "steps": [
                    {
                        "id": "step-1",
                        "name": "Step 1",
                        "description": format!("Run [[{workflow_id}]]")
                    }
                ]
            }))
            .expect("serialize workflow"),
        )
        .expect("write workflow");
    }

    (
        tmp,
        ToolContext::new(config, output_format),
        ConfigResetGuard,
    )
}
