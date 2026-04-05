# ACP timing probe — Codex `gpt-5.4` thoughtLevel

## Scope

Prompt used for every run:

```text
load maenifold, run recent_activity 10 includecontent true and synthesize those results
```

Execution path:

```text
maenifold tool --json AcpExec <payload>
```

Shared payload fields:

- `agent: "Codex"`
- `model: "gpt-5.4"`
- `mode: "prompt"`
- `autoGrantPermissions: true`
- `timeoutSeconds: 600`

Variable under test:

- `thoughtLevel: "low"`
- `thoughtLevel: "high"`
- `thoughtLevel: "xhigh"`

## Important caveat

The Codex ACP capability probe advertised `gpt-5.4` as an available model, but it did **not** advertise a `thoughtLevel` / `thought_level` config option. These runs therefore test whether forcing `thoughtLevel` through `AcpExec` has any observable latency effect in practice.

## Results

The original 6-run `low` vs `xhigh` matrix was stopped after the first pair because the timings were already non-monotonic and you asked for a `high` sanity check next.

| thoughtLevel | run | duration (ms) | duration (s) | result |
| --- | --- | ---: | ---: | --- |
| low | 1 | 119469 | 119.469 | success |
| xhigh | 1 | 67258 | 67.258 | success |
| high | 1 | 99626 | 99.626 | success |

## Duration histogram

Scale: **1 block = 5 seconds**

```text
low   | ████████████████████████ 119.469s
high  | ████████████████████     99.626s
xhigh | █████████████            67.258s
```

## Readout

- The timings are **not monotonic** with higher requested reasoning.
- `xhigh` was materially faster than both `high` and `low`.
- That makes the current evidence more consistent with **thoughtLevel being ignored, loosely mapped, or dominated by normal run-to-run/tool-use variance** than with a working "more reasoning = more latency" control.

## Practical conclusion

For **Codex + `gpt-5.4`**, `thoughtLevel` does not currently show a reliable timing signature from these probes. Treat it as **unproven / likely ineffective** until a larger controlled run demonstrates otherwise.
