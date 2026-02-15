import type { Plugin } from "@opencode-ai/plugin"

// T-OC-8: RTM FR-3.4
// In-memory by design: resets on OpenCode restart.
const _sequentialByProject = new Map<string, { sessionId: string; thoughtNumber: number }>()

// T-OC-8: RTM FR-3.4
const _SUMMARY_MAX_CHARS = 32000
const _PAYLOAD_MAX_CHARS = 50000
const _CLI_TIMEOUT_MS = 30_000
const _SERVICE = "opencode-plugin-persistence"

function _extractTextParts(parts: any[]): string {
  return (parts ?? [])
    .filter((p) => p?.type === "text" && typeof p.text === "string")
    .map((p) => p.text)
    .join("\n")
    .trim()
}

function _capText(
  text: string,
  maxChars: number,
  marker: string,
): { text: string; truncated: boolean; originalLength: number } {
  const originalLength = text.length
  if (originalLength <= maxChars) {
    return { text, truncated: false, originalLength }
  }

  const capped = text.slice(0, maxChars)
  return {
    text: `${capped}\n\n${marker}`,
    truncated: true,
    originalLength,
  }
}

function _containsWikiLink(text: string): boolean {
  return /\[\[[^\]]+\]\]/.test(text)
}

function _sanitizeForStorage(text: string): string {
  // Minimal hardening: normalize + strip control chars (keeps \n, \t).
  return text
    .normalize("NFKC")
    .replace(/[\u0000-\u0008\u000B\u000C\u000E-\u001F\u007F]/g, "")
    .trim()
}

// Parse maenifold --json output. Handles both proper JSON envelope and
// plain-text error responses (some SequentialThinking errors aren't JSON-wrapped).
function _parseMaenifoldOutput(stdout: string): {
  success: boolean
  sessionId?: string
  error?: string
} {
  const trimmed = stdout.trim()

  // Try JSON parse first (normal --json mode response)
  if (trimmed.startsWith("{")) {
    try {
      const parsed = JSON.parse(trimmed)
      if (parsed.success && parsed.data?.sessionId) {
        return { success: true, sessionId: parsed.data.sessionId }
      }
      // JSON error response
      return {
        success: false,
        error: parsed.error?.message ?? "Unknown maenifold error",
      }
    } catch {
      // JSON parse failed - fall through to plain text handling
    }
  }

  // Plain text fallback: some SequentialThinking errors return "ERROR: ..."
  // instead of JSON even with --json flag.
  if (trimmed.startsWith("ERROR:")) {
    return { success: false, error: trimmed }
  }

  // Try regex fallback for session ID in unstructured output
  const match = trimmed.match(/session-[\w-]+/)
  if (match) {
    return { success: true, sessionId: match[0] }
  }

  return { success: false, error: `Unexpected maenifold output: ${trimmed.slice(0, 200)}` }
}

export const PersistencePlugin: Plugin = async ({ project, client, directory, worktree }) => {
  return {
    event: async ({ event }) => {
      if (event.type !== "session.compacted") return

      // T-OC-8: RTM FR-3.4
      const sessionID = event.properties?.sessionID
      if (typeof sessionID !== "string") {
        await client.app.log({
          body: {
            service: _SERVICE,
            level: "warn",
            message: "session.compacted missing sessionID; skipping persistence",
            extra: { projectId: project?.id ?? null },
          },
        })
        return
      }

      const baseDir = worktree ?? directory
      const projectKey = project?.id ?? baseDir ?? "unknown-project"

      try {
        await client.app.log({
          body: {
            service: _SERVICE,
            level: "info",
            message: "session.compacted event received",
            extra: { projectId: project?.id ?? null, sessionID },
          },
        })

        // Extract compaction summary from session messages
        const result = await client.session.messages({ path: { id: sessionID } })
        const messages: any[] = Array.isArray(result) ? result : (result?.data ?? [])

        const summaryMsg = [...messages].reverse().find(
          (m) => m?.info?.role === "assistant" && m?.info?.summary === true,
        )

        const extractedSummary = summaryMsg ? _extractTextParts(summaryMsg.parts) : ""
        if (!extractedSummary) {
          await client.app.log({
            body: {
              service: _SERVICE,
              level: "warn",
              message: "Session compacted, but no summary message found.",
              extra: { projectId: project?.id ?? null, sessionID },
            },
          })
          await client.tui.showToast({
            body: {
              title: "Persistence",
              message: "Session compacted, but no summary message found.",
              variant: "warning",
              duration: 6000,
            },
          })
          return
        }

        // Sanitize and cap summary text
        const sanitizedSummary = _sanitizeForStorage(extractedSummary)
        const cappedSummary = _capText(
          sanitizedSummary,
          _SUMMARY_MAX_CHARS,
          `[TRUNCATED: summary exceeded ${_SUMMARY_MAX_CHARS} chars]`,
        )

        // Build SequentialThinking payload
        const prior = _sequentialByProject.get(projectKey)
        const thoughtNumber = prior ? prior.thoughtNumber + 1 : 0

        const summaryForMaenifold = cappedSummary.text
        const responsePrefix = _containsWikiLink(summaryForMaenifold)
          ? ""
          : "[[opencode]] [[compaction]]\n\n"
        const response = [
          responsePrefix,
          `## OpenCode compaction summary (untrusted)`,
          ``,
          "```",
          summaryForMaenifold,
          "```",
        ].join("\n")

        // totalThoughts set to 0 = unbounded session (no artificial ceiling).
        // nextThoughtNeeded: true signals the session stays open for future compactions.
        const payload: Record<string, unknown> = {
          response,
          nextThoughtNeeded: true,
          thoughtNumber,
          totalThoughts: 0,
        }

        if (prior) {
          payload.sessionId = prior.sessionId
        }

        const payloadJson = JSON.stringify(payload)

        if (payloadJson.length > _PAYLOAD_MAX_CHARS) {
          await client.app.log({
            body: {
              service: _SERVICE,
              level: "warn",
              message: "SequentialThinking payload too large; skipping",
              extra: {
                projectId: project?.id ?? null,
                sessionID,
                thoughtNumber,
                payloadChars: payloadJson.length,
              },
            },
          })
          await client.tui.showToast({
            body: {
              title: "Persistence",
              message: `Skipping maenifold: payload too large (${payloadJson.length} chars).`,
              variant: "warning",
              duration: 8000,
            },
          })
          return
        }

        // Invoke maenifold CLI with --json for structured output and timeout for safety
        const proc = Bun.spawn(
          ["maenifold", "--tool", "SequentialThinking", "--payload", payloadJson, "--json"],
          { stdout: "pipe", stderr: "pipe", timeout: _CLI_TIMEOUT_MS },
        )

        const [stdout, stderr, exitCode] = await Promise.all([
          new Response(proc.stdout).text(),
          new Response(proc.stderr).text(),
          proc.exited,
        ])

        // Detect timeout (process killed by signal)
        if (proc.signalCode) {
          throw new Error(
            `maenifold CLI killed by ${proc.signalCode} (timeout after ${_CLI_TIMEOUT_MS}ms)`,
          )
        }

        if (exitCode !== 0) {
          throw new Error(
            `maenifold SequentialThinking failed (exit ${exitCode}): ${stderr || stdout}`,
          )
        }

        // Parse structured JSON output (with plain-text error fallback)
        const parsed = _parseMaenifoldOutput(stdout)

        if (!parsed.success) {
          throw new Error(`maenifold SequentialThinking error: ${parsed.error}`)
        }

        const updatedSessionId = parsed.sessionId ?? prior?.sessionId
        if (!updatedSessionId) {
          throw new Error("Failed to extract maenifold sessionId from output")
        }

        _sequentialByProject.set(projectKey, {
          sessionId: updatedSessionId,
          thoughtNumber,
        })

        await client.app.log({
          body: {
            service: _SERVICE,
            level: "info",
            message: "Compaction summary persisted to maenifold SequentialThinking",
            extra: {
              projectId: project?.id ?? null,
              sessionID,
              thoughtNumber,
              maenifoldSessionId: updatedSessionId,
              summaryTruncated: cappedSummary.truncated,
            },
          },
        })

        await client.tui.showToast({
          body: {
            title: "Persistence",
            message: `Saved compaction summary (thought ${thoughtNumber})`,
            variant: "success",
            duration: 4000,
          },
        })
      } catch (err) {
        // Best-effort structured log; do not throw from plugin.
        try {
          await client.app.log({
            body: {
              service: _SERVICE,
              level: "error",
              message: "Compaction persistence failed",
              extra: {
                projectId: project?.id ?? null,
                sessionID,
                error: err instanceof Error ? err.message : String(err),
              },
            },
          })
        } catch {
          // ignore logging failure
        }

        await client.tui.showToast({
          body: {
            title: "Persistence",
            message: `Compaction persistence failed: ${err instanceof Error ? err.message : String(err)}`,
            variant: "error",
            duration: 8000,
          },
        })
      }
    },
  }
}
