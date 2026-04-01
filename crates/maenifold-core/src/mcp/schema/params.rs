#![allow(dead_code)] // Schema-only types — fields are read by schemars, not by application code

use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{Map, Value};

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

fn default_zero_u32() -> u32 {
    0
}

fn default_one_u32() -> u32 {
    1
}

fn default_two_u32() -> u32 {
    2
}

fn default_five_u32() -> u32 {
    5
}

fn default_eight_u32() -> u32 {
    8
}

fn default_ten_u32() -> u32 {
    10
}

fn default_twenty_u32() -> u32 {
    20
}

fn default_thirty_u32() -> u32 {
    30
}

fn default_fifty_u32() -> u32 {
    50
}

fn default_one_hundred_fifty_u32() -> u32 {
    150
}

fn default_one_thousand_u32() -> u32 {
    1000
}

fn default_zero_f64() -> f64 {
    0.0
}

fn default_point_seven() -> f64 {
    0.7
}

fn default_search_mode() -> SearchMode {
    SearchMode::Hybrid
}

fn default_recent_activity_filter() -> RecentActivityFilter {
    RecentActivityFilter::All
}

fn default_edge_type() -> PipelineEdgeType {
    PipelineEdgeType::Success
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WriteMemoryParams {
    /// Title for this memory file
    pub title: String,
    /// Content with [[Concept Names]]; at least one [[concept]] is required
    pub content: String,
    /// Optional folder path for organizing files
    #[serde(default)]
    pub folder: Option<String>,
    /// Optional tags for categorizing this file
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadMemoryParams {
    /// Memory file identifier: URI or exact title
    pub identifier: String,
    /// Return checksum with content for safe editing
    #[serde(default = "default_true")]
    #[schemars(default = "default_true")]
    pub include_checksum: bool,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EditMemoryOperation {
    Append,
    Prepend,
    FindReplace,
    ReplaceSection,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditMemoryParams {
    /// Memory file identifier: URI or exact title
    pub identifier: String,
    /// Edit operation to apply
    pub operation: EditMemoryOperation,
    /// Content to add or replace; must include at least one [[concept]]
    pub content: String,
    /// File checksum from the last read for stale-edit protection
    #[serde(default)]
    pub checksum: Option<String>,
    /// Text to find when operation is find_replace
    #[serde(default)]
    pub find_text: Option<String>,
    /// Section name when operation is replace_section
    #[serde(default)]
    pub section_name: Option<String>,
    /// Expected match count when operation is find_replace
    #[serde(default)]
    pub expected_count: Option<u32>,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteMemoryParams {
    /// Memory file identifier: URI or exact title
    pub identifier: String,
    /// Confirm deletion of the file
    #[serde(default)]
    pub confirm: bool,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MoveMemoryParams {
    /// Source memory file identifier: URI or exact title
    pub source: String,
    /// Destination file name or relative path
    pub destination: String,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
pub enum SearchMode {
    Hybrid,
    Semantic,
    FullText,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SearchMemoriesParams {
    /// Search query over file contents
    pub query: String,
    /// Search mode
    #[serde(default = "default_search_mode")]
    #[schemars(default = "default_search_mode")]
    pub mode: SearchMode,
    /// Maximum files to return
    #[serde(default = "default_ten_u32")]
    #[schemars(default = "default_ten_u32")]
    pub page_size: u32,
    /// Page number for results
    #[serde(default = "default_one_u32")]
    #[schemars(default = "default_one_u32")]
    pub page: u32,
    /// Optional folder path filter under the memory root
    #[serde(default)]
    pub folder: Option<String>,
    /// Optional tag filter; all provided tags must match
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// Minimum score threshold
    #[serde(default = "default_zero_f64")]
    #[schemars(default = "default_zero_f64")]
    pub min_score: f64,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExtractConceptsFromFileParams {
    /// Memory file identifier: URI or exact title
    pub identifier: String,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListMemoriesParams {
    /// Optional directory path relative to the memory root
    #[serde(default)]
    pub path: Option<String>,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncParams {
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BuildContextParams {
    /// Concept name to build context around
    pub concept_name: String,
    /// How many graph hops to traverse
    #[serde(default = "default_two_u32")]
    #[schemars(default = "default_two_u32")]
    pub depth: u32,
    /// Maximum related entities to return
    #[serde(default = "default_twenty_u32")]
    #[schemars(default = "default_twenty_u32")]
    pub max_entities: u32,
    /// Include content previews in the context response
    #[serde(default)]
    pub include_content: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VisualizeParams {
    /// Central concept for the visualization
    pub concept_name: String,
    /// Graph traversal depth
    #[serde(default = "default_two_u32")]
    #[schemars(default = "default_two_u32")]
    pub depth: u32,
    /// Maximum nodes to include in the diagram
    #[serde(default = "default_thirty_u32")]
    #[schemars(default = "default_thirty_u32")]
    pub max_nodes: u32,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RepairConceptsParams {
    /// Delimited concept variants to replace
    pub concepts_to_replace: String,
    /// Canonical concept to keep after consolidation
    pub canonical_concept: String,
    /// Optional folder under the memory root to process
    #[serde(default)]
    pub folder: Option<String>,
    /// Preview changes without modifying files
    #[serde(default = "default_true")]
    #[schemars(default = "default_true")]
    pub dry_run: bool,
    /// Create WikiLinks from plain text instead of replacing existing ones
    #[serde(default)]
    pub create_wiki_links: bool,
    /// Minimum semantic similarity required for safe consolidation
    #[serde(default = "default_point_seven")]
    #[schemars(default = "default_point_seven")]
    pub min_semantic_similarity: f64,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnalyzeConceptCorruptionParams {
    /// Concept family to analyze, such as tool or agent
    pub concept_family: String,
    /// Maximum variants to include
    #[serde(default = "default_fifty_u32")]
    #[schemars(default = "default_fifty_u32")]
    pub max_results: u32,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FindSimilarConceptsParams {
    /// Concept name to search for related concepts
    pub concept_name: String,
    /// Maximum number of similar concepts to return
    #[serde(default = "default_ten_u32")]
    #[schemars(default = "default_ten_u32")]
    pub max_results: u32,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SequentialThinkingParams {
    /// Main thought content; must include [[WikiLinks]] unless cancelling
    #[serde(default)]
    pub response: Option<String>,
    /// Whether another thought is needed after this one
    #[serde(default)]
    pub next_thought_needed: bool,
    /// Current thought number
    #[serde(default = "default_zero_u32")]
    #[schemars(default = "default_zero_u32")]
    pub thought_number: u32,
    /// Current estimate for total thoughts
    #[serde(default = "default_zero_u32")]
    #[schemars(default = "default_zero_u32")]
    pub total_thoughts: u32,
    /// Existing sequential thinking session identifier
    #[serde(default)]
    pub session_id: Option<String>,
    /// Cancel the session instead of adding a thought
    #[serde(default)]
    pub cancel: bool,
    /// Optional meta-thoughts; include [[WikiLinks]] when provided
    #[serde(default)]
    pub thoughts: Option<String>,
    /// Marks this thought as a revision of prior thinking
    #[serde(default)]
    pub is_revision: bool,
    /// Thought number that this revision updates
    #[serde(default)]
    pub revises_thought: Option<u32>,
    /// Thought number where this branch diverges
    #[serde(default)]
    pub branch_from_thought: Option<u32>,
    /// Branch identifier for multi-agent coordination
    #[serde(default)]
    pub branch_id: Option<String>,
    /// Whether more thoughts are needed than originally estimated
    #[serde(default)]
    pub needs_more_thoughts: bool,
    /// Optional analysis type annotation such as bug or architecture
    #[serde(default)]
    pub analysis_type: Option<String>,
    /// Optional parent workflow session identifier
    #[serde(default)]
    pub parent_workflow_id: Option<String>,
    /// Structured ConfessionReport used when completing the session
    #[serde(default)]
    pub conclusion: Option<String>,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum WorkflowIdValue {
    String(String),
    List(Vec<String>),
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowStatus {
    Completed,
    Cancelled,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowParams {
    /// Existing workflow session identifier to continue
    #[serde(default)]
    pub session_id: Option<String>,
    /// Workflow identifier or list of workflow identifiers to start
    #[serde(default)]
    pub workflow_id: Option<WorkflowIdValue>,
    /// Response to the current step; include [[WikiLinks]]
    #[serde(default)]
    pub response: Option<String>,
    /// Ambient or meta-thoughts; include [[WikiLinks]]
    #[serde(default)]
    pub thoughts: Option<String>,
    /// Terminal status for the workflow session
    #[serde(default)]
    pub status: Option<WorkflowStatus>,
    /// Structured ConfessionReport used when completing the session
    #[serde(default)]
    pub conclusion: Option<String>,
    /// Display the workflow queue status
    #[serde(default)]
    pub view: bool,
    /// Workflow identifier or identifiers to append to the queue
    #[serde(default)]
    pub append: Option<String>,
    /// SequentialThinking session identifier registered as a submachine
    #[serde(default)]
    pub submachine_session_id: Option<String>,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum RecentActivityFilter {
    Thinking,
    Memory,
    Assumptions,
    All,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecentActivityParams {
    /// Maximum number of results to return
    #[serde(default = "default_ten_u32")]
    #[schemars(default = "default_ten_u32")]
    pub limit: u32,
    /// Activity filter
    #[serde(default = "default_recent_activity_filter")]
    #[schemars(default = "default_recent_activity_filter")]
    pub filter: RecentActivityFilter,
    /// Positive timespan such as 24.00:00:00
    #[serde(default)]
    pub timespan: Option<String>,
    /// Include full H2 section content instead of headers only
    #[serde(default)]
    pub include_content: bool,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum AssumptionLedgerAction {
    Append,
    Update,
    Read,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum AssumptionStatus {
    Active,
    Validated,
    Invalidated,
    Refined,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssumptionLedgerParams {
    /// Ledger action to perform
    pub action: AssumptionLedgerAction,
    /// Assumption statement for append operations
    #[serde(default)]
    pub assumption: Option<String>,
    /// Workflow or thinking session reference
    #[serde(default)]
    pub context: Option<String>,
    /// Planned validation method
    #[serde(default)]
    pub validation_plan: Option<String>,
    /// Confidence level such as high, medium, or low
    #[serde(default)]
    pub confidence: Option<String>,
    /// Concept tags for knowledge graph integration
    #[serde(default)]
    pub concepts: Option<Vec<String>>,
    /// Memory URI for update or read actions
    #[serde(default)]
    pub uri: Option<String>,
    /// Updated assumption status
    #[serde(default)]
    pub status: Option<AssumptionStatus>,
    /// Additional notes for update operations
    #[serde(default)]
    pub notes: Option<String>,
    /// Return help documentation instead of executing
    #[serde(default)]
    pub learn: bool,
}

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
pub struct StartWatcherParams {
    /// Debounce interval in milliseconds
    #[serde(default = "default_one_hundred_fifty_u32")]
    #[schemars(default = "default_one_hundred_fifty_u32")]
    pub debounce_ms: u32,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StopWatcherParams {}

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
pub struct ListAgentsParams {}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AcpPromptParams {
    /// ACP agent name from agents.json
    pub agent: String,
    /// Prompt text to send to the agent
    pub prompt: String,
    /// Optional model override for the session
    #[serde(default)]
    pub model: Option<String>,
    /// Optional session mode override
    #[serde(default)]
    pub mode: Option<String>,
    /// Optional session timeout in seconds
    #[serde(default)]
    pub timeout_seconds: Option<u32>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PipelineParams {
    /// Existing pipeline session identifier
    #[serde(default)]
    pub session_id: Option<String>,
    /// Pipeline definition with nodes[] and edges[]
    #[serde(default)]
    pub definition: Option<PipelineDefinitionSchema>,
    /// Max concurrent active nodes
    #[serde(default = "default_eight_u32")]
    #[schemars(default = "default_eight_u32")]
    pub max_concurrency: u32,
    /// Start autonomous execution for an existing pipeline
    #[serde(default)]
    pub run: bool,
    /// Ambient prompt for autonomous pipeline execution
    #[serde(default)]
    pub prompt: Option<String>,
    /// Pause a running pipeline after active nodes drain
    #[serde(default)]
    pub pause: bool,
    /// Resume a paused pipeline
    #[serde(default)]
    pub resume: bool,
    /// Cancel a running pipeline and stop future execution
    #[serde(default)]
    pub cancel: bool,
    /// Return current pipeline state
    #[serde(default)]
    pub view: bool,
    /// Claim ready nodes as active
    #[serde(default)]
    pub dispatch: bool,
    /// List all pipeline sessions with status summaries
    #[serde(default)]
    pub list: bool,
    /// Rewind the specified nodeIds and all downstream nodes
    #[serde(default)]
    pub rewind: Option<PipelineRewindSchema>,
    /// Delete the pipeline session and all associated workflow sessions
    #[serde(default)]
    pub delete: bool,
    /// Add new nodes and edges onto an existing pipeline session
    #[serde(default)]
    pub graft: Option<PipelineGraftSchema>,
    /// Merge another pipeline session into this session using additional linking edges
    #[serde(default)]
    pub join: Option<PipelineJoinSchema>,
    /// Clone the selected node and downstream subgraph into a new pipeline session
    #[serde(default)]
    pub fork: Option<PipelineForkSchema>,
    /// Target node for status updates or restart
    #[serde(default)]
    pub node_id: Option<String>,
    /// Node status update
    #[serde(default)]
    pub status: Option<PipelineNodeStatusUpdate>,
    /// Conclusion text when completing a node
    #[serde(default)]
    pub conclusion: Option<String>,
    /// Restart an abandoned/stuck node
    #[serde(default)]
    pub restart: bool,
    /// Return help documentation
    #[serde(default)]
    pub learn: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PipelineDefinitionSchema {
    /// Pipeline nodes to execute
    pub nodes: Vec<PipelineNodeSchema>,
    /// Pipeline edges connecting nodes
    pub edges: Vec<PipelineEdgeSchema>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PipelineRewindSchema {
    /// Node identifiers to rewind before cascading downstream resets
    pub node_ids: Vec<String>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PipelineGraftSchema {
    /// New pipeline nodes to append
    pub nodes: Vec<PipelineNodeSchema>,
    /// New pipeline edges to add
    pub edges: Vec<PipelineEdgeSchema>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PipelineJoinSchema {
    /// Pipeline session identifier to merge into the target pipeline
    pub source_session_id: String,
    /// Edges that connect the source pipeline into the target pipeline
    pub edges: Vec<PipelineEdgeSchema>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PipelineForkSchema {
    /// Node identifier where the forked subgraph starts
    pub from_node_id: String,
    /// Optional pipeline session identifier for the forked pipeline
    #[serde(default)]
    pub new_session_id: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum PipelineNodeStatusUpdate {
    Completed,
    Abandoned,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PipelineEdgeSchema {
    /// Upstream node identifier
    pub source: String,
    /// Downstream node identifier
    pub target: String,
    /// Edge type controlling downstream activation
    #[serde(rename = "type", default = "default_edge_type")]
    #[schemars(default = "default_edge_type")]
    pub edge_type: PipelineEdgeType,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum PipelineEdgeType {
    Success,
    Always,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PipelineAgentConfigSchema {
    /// Model override for the agent session
    #[serde(default)]
    pub model: Option<String>,
    /// Session mode override for the agent session
    #[serde(default)]
    pub mode: Option<String>,
    /// Thought level override for the agent session
    #[serde(default)]
    pub thought_level: Option<String>,
    /// Arbitrary config options forwarded to the agent session
    #[serde(default)]
    pub config_options: Option<Map<String, Value>>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum PipelineNodeSchema {
    Workflow(PipelineWorkflowNodeSchema),
    Prompt(PipelinePromptNodeSchema),
    SequentialThinking(PipelineSequentialThinkingNodeSchema),
    Tool(PipelineToolNodeSchema),
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PipelineWorkflowNodeSchema {
    /// New node identifier
    pub id: String,
    /// Workflow node type
    #[serde(rename = "type")]
    pub kind: PipelineWorkflowNodeType,
    /// Workflow definition to run
    pub workflow_id: String,
    /// Optional ACP client
    #[serde(default)]
    pub acp_client: Option<String>,
    /// Optional prompt for workflow nodes
    #[serde(default)]
    pub prompt: Option<String>,
    /// Optional timeout in seconds
    #[serde(default)]
    pub timeout: Option<u32>,
}

#[derive(Deserialize, JsonSchema)]
pub enum PipelineWorkflowNodeType {
    #[serde(rename = "workflow")]
    Workflow,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PipelinePromptNodeSchema {
    /// New node identifier
    pub id: String,
    /// Node type 'prompt'
    #[serde(rename = "type")]
    pub kind: PipelinePromptNodeType,
    /// Prompt text for prompt nodes
    pub prompt: String,
    /// ACP agent for prompt nodes
    pub agent: String,
    /// Tool mode for prompt nodes
    #[serde(default)]
    pub tool_mode: Option<String>,
    /// Model override for prompt nodes
    #[serde(default)]
    pub model: Option<String>,
    /// Thought level override for prompt nodes
    #[serde(default)]
    pub thought_level: Option<String>,
    /// Allowed tools override for prompt nodes
    #[serde(default)]
    pub allowed_tools: Option<Vec<String>>,
    /// Maximum turns for prompt nodes
    #[serde(default)]
    pub max_turns: Option<u32>,
    /// Optional timeout in seconds
    #[serde(default)]
    pub timeout: Option<u32>,
    #[serde(default)]
    pub agent_config: Option<PipelineAgentConfigSchema>,
}

#[derive(Deserialize, JsonSchema)]
pub enum PipelinePromptNodeType {
    #[serde(rename = "prompt")]
    Prompt,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PipelineSequentialThinkingNodeSchema {
    /// New node identifier
    pub id: String,
    /// Node type 'sequentialThinking'
    #[serde(rename = "type")]
    pub kind: PipelineSequentialThinkingNodeType,
    /// Topic for sequential thinking nodes
    pub topic: String,
    /// Thought count for thinking nodes
    pub total_thoughts: u32,
    /// Analysis type for thinking nodes
    #[serde(default)]
    pub analysis_type: Option<String>,
    /// ACP agent for sequential thinking nodes
    pub agent: String,
    /// Model override for sequential thinking nodes
    #[serde(default)]
    pub model: Option<String>,
    /// Thought level override for sequential thinking nodes
    #[serde(default)]
    pub thought_level: Option<String>,
    /// Allowed tools override for sequential thinking nodes
    #[serde(default)]
    pub allowed_tools: Option<Vec<String>>,
    /// Maximum turns for sequential thinking nodes
    #[serde(default)]
    pub max_turns: Option<u32>,
    /// Optional timeout in seconds
    #[serde(default)]
    pub timeout: Option<u32>,
    #[serde(default)]
    pub agent_config: Option<PipelineAgentConfigSchema>,
}

#[derive(Deserialize, JsonSchema)]
pub enum PipelineSequentialThinkingNodeType {
    #[serde(rename = "sequentialThinking")]
    SequentialThinking,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PipelineToolNodeSchema {
    /// New node identifier
    pub id: String,
    /// Node type 'tool'
    #[serde(rename = "type")]
    pub kind: PipelineToolNodeType,
    /// MCP tool name for tool nodes
    pub tool_name: String,
    /// Tool arguments for tool nodes
    #[serde(default)]
    pub tool_args: Option<Map<String, Value>>,
    /// Optional ACP agent for tool nodes
    #[serde(default)]
    pub agent: Option<String>,
    /// Model override for tool nodes
    #[serde(default)]
    pub model: Option<String>,
    /// Thought level override for tool nodes
    #[serde(default)]
    pub thought_level: Option<String>,
    /// Allowed tools override for tool nodes
    #[serde(default)]
    pub allowed_tools: Option<Vec<String>>,
    /// Maximum turns for tool nodes
    #[serde(default)]
    pub max_turns: Option<u32>,
    #[serde(default)]
    pub agent_config: Option<PipelineAgentConfigSchema>,
}

#[derive(Deserialize, JsonSchema)]
pub enum PipelineToolNodeType {
    #[serde(rename = "tool")]
    Tool,
}
