#![allow(dead_code)] // Schema-only types — fields are read by schemars, not by application code

use super::params::{
    default_five_u32, default_one_thousand_u32, default_prompt_mode, default_true,
};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemoryStatusParams {
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetConfigParams {
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetHelpParams {
    /// Tool name to load help for
    pub tool_name: String,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateAssetsParams {
    /// Preview changes without modifying files
    #[serde(default = "default_true")]
    #[schemars(default = "default_true")]
    pub dry_run: bool,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RunFullBenchmarkParams {
    /// Benchmark iterations per category
    #[serde(default = "default_five_u32")]
    #[schemars(default = "default_five_u32")]
    pub iterations: u32,
    /// Maximum test files to use during benchmarks
    #[serde(default = "default_one_thousand_u32")]
    #[schemars(default = "default_one_thousand_u32")]
    pub max_test_files: u32,
    /// Include expensive deep traversal benchmarks
    #[serde(default = "default_true")]
    #[schemars(default = "default_true")]
    pub include_deep_traversal: bool,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum AdoptAssetType {
    Role,
    Color,
    Perspective,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AdoptParams {
    /// Asset type to adopt
    pub r#type: AdoptAssetType,
    /// Asset identifier or filename without the .json extension
    pub identifier: String,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ListAssetsType {
    Workflow,
    Role,
    Color,
    Perspective,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListAssetsParams {
    /// Optional asset type to list
    #[serde(default)]
    pub r#type: Option<ListAssetsType>,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadMcpResourceParams {
    /// Resource URI such as asset://catalog
    pub uri: String,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListSessionsParams {
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum TeamsMonitorAction {
    Enable,
    Disable,
    Status,
    Configure,
    Authenticate,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeamsMonitorParams {
    /// Teams monitor operation to perform
    pub action: TeamsMonitorAction,
    /// Azure AD application client ID for device code authentication
    #[serde(default)]
    pub client_id: Option<String>,
    /// Azure AD tenant ID or tenant name for device code authentication
    #[serde(default)]
    pub tenant_id: Option<String>,
    /// Keywords that trigger a monitor match during configure
    #[serde(default)]
    pub keywords: Option<Vec<String>>,
    /// Optional OAuth scopes for device code authentication
    #[serde(default)]
    pub scopes: Option<Vec<String>>,
    /// Optional sender allow-list to require for matches during configure. Pass null to clear.
    #[serde(default)]
    pub sender_allow_list: Option<Vec<String>>,
    /// Optional sender deny-list to reject during configure. Pass null to clear.
    #[serde(default)]
    pub sender_deny_list: Option<Vec<String>>,
    /// Per-conversation cooldown in seconds during configure
    #[serde(default)]
    pub cooldown_seconds: Option<u64>,
    /// ACP agent to dispatch when a match occurs during configure. Pass null to clear.
    #[serde(default)]
    pub dispatch_agent: Option<String>,
}

/// Additional content blocks included alongside an ACP prompt request.
#[derive(Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum AcpExecContextBlock {
    /// Plain text context block.
    Text(AcpExecTextContext),
    /// Base64-encoded image context block.
    Image(AcpExecImageContext),
    /// Embedded resource context block.
    Resource(AcpExecResourceContext),
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AcpExecTextContext {
    /// Additional text supplied with the prompt.
    pub text: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AcpExecImageContext {
    /// Base64-encoded image data.
    pub data: String,
    /// Media type of the image, for example `image/png`.
    #[serde(alias = "mimeType")]
    pub media_type: String,
    /// Optional source URI for the image.
    #[serde(default)]
    pub uri: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AcpExecResourceContext {
    /// Absolute or logical URI for the embedded resource.
    pub uri: String,
    /// Text file content. Provide exactly one of `text` or `blob`.
    #[serde(default)]
    pub text: Option<String>,
    /// Base64-encoded binary content. Provide exactly one of `text` or `blob`.
    #[serde(default)]
    pub blob: Option<String>,
    /// Optional media type for the resource, for example `text/plain`.
    #[serde(default, alias = "mimeType")]
    pub media_type: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AcpExecParams {
    /// ACP agent name from maenifold.json
    pub agent: String,
    /// Optional ACP session ID to load instead of creating a new session
    #[serde(default)]
    pub session_id: Option<String>,
    /// Optional working directory for the ACP session
    #[serde(default)]
    pub cwd: Option<String>,
    /// Whether ACP permission requests should be auto-granted by the native client.
    pub auto_grant_permissions: bool,
    /// Execution mode
    #[serde(default = "default_prompt_mode")]
    #[schemars(default = "default_prompt_mode")]
    pub mode: Option<String>,
    /// Prompt text to send to the agent (required for prompt and workflow modes)
    #[serde(default)]
    pub prompt: Option<String>,
    /// Topic for sequential thinking (required for think mode)
    #[serde(default)]
    pub topic: Option<String>,
    /// Estimated number of thoughts for think mode
    #[serde(default)]
    pub total_thoughts: Option<u32>,
    /// Workflow identifier (required for workflow mode)
    #[serde(default)]
    pub workflow_id: Option<String>,
    /// Optional model override for the session
    #[serde(default)]
    pub model: Option<String>,
    /// Optional thinking effort level, applied via session/set_config_option if the agent advertises it.
    #[serde(default)]
    pub thought_level: Option<String>,
    /// Optional allowed tools override
    #[serde(default)]
    pub allowed_tools: Option<Vec<String>>,
    /// Optional max turns override
    #[serde(default)]
    pub max_turns: Option<u32>,
    /// Optional execution timeout in seconds. When set, cancels the operation
    /// if it exceeds this duration. Connection timeout is configured separately
    /// in maenifold.json settings.
    #[serde(default)]
    pub timeout_seconds: Option<u32>,
}
