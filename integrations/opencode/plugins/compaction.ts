import type { Plugin } from "@opencode-ai/plugin"

// T-SPEC-3: RTM FR-3.5 — Inject WikiLink tagging context into compaction prompt
export const CompactionPlugin: Plugin = async (_ctx) => {
  return {
    "experimental.session.compacting": async (_input, output) => {
      // Inject additional context into the compaction prompt
      output.context.push(`
When writing your summaries for compaction/continuation, please adhere to the following wiki-style tagging guidelines.  
WikiLinks become graph nodes. Bad tagging = graph corruption = broken context recovery.

What NOT to tag:
When planning your summary think about what would be useful to link to in a wiki for future reference.
Tag only the most important concepts and topics discussed in the session. 
Never tag low signal or meta-terms. These degrade the graph:
BANNED: [[concept]], [[concepts]], [[WikiLink]], [[WikiLinks]], [[tool]], [[tools]], [[example]], [[thing]], [[item]], [[entity]], [[file]], [[agent]], [[system]], [[test]], [[data]], [[model]]
USE INSTEAD: Specific terms like [[authentication]], [[JWT]], [[vector-embeddings]], [[MCP]]
If the topic isn't worth writing a detailed wiki page about, don't tag it.

Tagging guidelines:
- Double brackets: [[WikiLink]] never [WikiLink]
- Normalized to lowercase-with-hyphens internally
- PRIMARY concept only: [[MCP]] not [[MCP-server]]
- GENERAL terms: [[authentication]] not [[auth-system]]
- NO file paths, code elements, or trivial words ([[the]], [[a]])
- TAG substance: [[machine-learning]], [[GraphRAG]], [[vector-embeddings]]
- REUSE existing concepts before inventing near-duplicates (guard fragmentation)
- HYPHENATE multiword: [[null-reference-exception]] not [[Null Reference Exception]]

Anti-patterns (silently normalized but avoid):
- Underscores: [[my_concept]] → use [[my-concept]]
- Slashes: [[foo/bar]] → use [[foo-bar]] or separate concepts
- Double hyphens: [[foo--bar]] → use [[foo-bar]]
- Leading/trailing hyphens: [[-concept-]] → use [[concept]]

Example: Fixed [[null-reference-exception]] in [[authentication]] using [[JWT]]
`)
    }
  }
}
