import { joinSession } from "@github/copilot-sdk/extension";

const SOURCE_EXTS = /\.(rs|toml)$/i;
const SOURCE_GLOBS = /\*\.\{?(rs|toml)[,}]/i;

function targetsSourceCode(toolArgs) {
  const glob = toolArgs?.glob || "";
  const path = toolArgs?.path || "";
  const type = toolArgs?.type || "";
  const pattern = toolArgs?.pattern || "";

  if (SOURCE_GLOBS.test(glob)) return true;
  if (SOURCE_EXTS.test(glob)) return true;
  if (SOURCE_EXTS.test(path)) return true;
  if (["rust", "rs", "toml"].includes(type)) return true;
  if (SOURCE_GLOBS.test(pattern)) return true;

  return false;
}

const LSP_GUIDANCE = `STOP — LSP is mandatory for source code navigation in this project.

Use these LSP operations instead of grep/glob on source code:
  • findReferences — find all usages of a symbol (use BEFORE deleting any file or symbol)
  • goToDefinition — find where a symbol is defined
  • incomingCalls — find what calls a function
  • outgoingCalls — find what a function calls
  • workspaceSymbol — search symbols by name across the workspace
  • hover — get type info and documentation
  • documentSymbol — list all symbols in a file

grep/glob are allowed for non-source files (docs, config, JSON, YAML, markdown).
If LSP is unavailable, stop and tell the user.`;

const session = await joinSession({
  hooks: {
    onPreToolUse: async (input) => {
      if (input.toolName === "grep" && targetsSourceCode(input.toolArgs || {})) {
        return {
          permissionDecision: "deny",
          permissionDecisionReason: LSP_GUIDANCE,
        };
      }

      if (input.toolName === "glob") {
        const pat = input.toolArgs?.pattern || "";
        if (SOURCE_EXTS.test(pat) || SOURCE_GLOBS.test(pat)) {
          return {
            permissionDecision: "deny",
            permissionDecisionReason: LSP_GUIDANCE,
          };
        }
      }
    },

    onSessionStart: async () => {
      await session.log("LSP enforcer loaded — grep/glob on Rust source code will be denied", {
        level: "info",
      });
    },
  },
  tools: [],
});
