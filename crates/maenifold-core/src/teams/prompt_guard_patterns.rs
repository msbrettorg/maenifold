//! Prompt-injection pattern table — port of agency-cowork
//! `scripts/prompt_guard.py` `_PATTERNS` (L44-249) and `_ALLOWLIST` (L257-283).
//!
//! Patterns use `(?ix)` (case-insensitive + extended whitespace-ignore mode)
//! supported by the Rust `regex` crate. Each entry cites its agency-cowork
//! line range so the two implementations stay diff-able.

use std::sync::LazyLock;

use regex::Regex;

use super::prompt_guard::Severity;

pub(super) struct PatternEntry {
    pub name: &'static str,
    pub description: &'static str,
    pub severity: Severity,
    pub regex: Regex,
}

fn re(pattern: &str) -> Regex {
    Regex::new(pattern).expect("prompt_guard pattern must compile")
}

pub(super) static PATTERNS: LazyLock<Vec<PatternEntry>> = LazyLock::new(|| {
    vec![
        // CRITICAL: System prompt override
        PatternEntry { // py L46-56
            name: "ignore_instructions",
            description: "Attempt to override system instructions",
            severity: Severity::Critical,
            regex: re(r"(?ix)
                (?: ignore | disregard | forget | override | bypass ) \s+
                (?: all \s+ )?
                (?: previous | prior | above | earlier | existing | your ) \s+
                (?: instructions | prompts? | rules? | guidelines? | constraints? | directives? )
            "),
        },
        PatternEntry { // py L57-66
            name: "new_system_prompt",
            description: "Attempt to inject a new system prompt",
            severity: Severity::Critical,
            regex: re(r"(?ix)
                (?: new | updated? | revised? | replacement ) \s+
                (?: system \s+ )?
                (?: prompt | instructions? | directives? )
                \s* [:\-]
            "),
        },
        PatternEntry { // py L67-77
            name: "you_are_now",
            description: "Attempt to redefine agent identity",
            severity: Severity::Critical,
            regex: re(r"(?ix)
                (?: you \s+ are \s+ now
                  | from \s+ now \s+ on \s+ you \s+ are
                  | your \s+ new \s+ (?: role | identity | persona ) \s+ is
                  | you \s+ (?: must | should | shall ) \s+ now \s+
                    (?: act | behave | respond ) \s+ as )
            "),
        },
        PatternEntry { // py L78-89
            name: "instruction_delimiter",
            description: "Fake instruction delimiter tags",
            severity: Severity::Critical,
            regex: re(r"(?ix)
                (?: < \s* /? (?: system | s | SYS | INST | instructions? | rules? ) \s* >
                  | \[ \s* (?: INST | SYS (?: TEM )? | SYSTEM_PROMPT ) \s* \]
                  | << \s* SYS \s* >>
                  | \#\#\# \s* (?: System | SYSTEM | Instructions? ) \s* (?: Prompt )? : )
            "),
        },
        // HIGH: Role / tool injection
        PatternEntry { // py L92-101
            name: "roleplay_injection",
            description: "Attempt to make agent adopt a different persona",
            severity: Severity::High,
            regex: re(r"(?ix)
                (?: pretend | imagine | act \s+ as \s+ if | roleplay | simulate
                  | play \s+ the \s+ role ) \s+
                (?: you \s+ are | that \s+ you | to \s+ be | you're ) \s+
            "),
        },
        PatternEntry { // py L102-110
            name: "tool_invocation",
            description: "Attempt to direct agent to invoke specific tools",
            severity: Severity::High,
            regex: re(r"(?ix)
                (?: call | invoke | execute | run | trigger | use ) \s+
                (?: the \s+ )?
                (?: function | tool | command | api | endpoint | script ) \s+
            "),
        },
        PatternEntry { // py L111-126 — destructive command list ported verbatim
            name: "shell_injection",
            description: "Shell metacharacters or dangerous command patterns",
            severity: Severity::High,
            regex: re(r"(?ix)
                (?: ^ | [\s;|&] )
                (?: (?: && | \|\| | ; ) \s* \w+
                  | ` [^`]+ `
                  | \$ \( [^)]+ \)
                  | > \s* /\w+ | >> \s* /\w+
                  | (?: rm | del | format | shutdown | kill | stop-process ) \s )
            "),
        },
        PatternEntry { // py L127-137
            name: "data_exfiltration",
            description: "Instruction to exfiltrate data via outbound channels",
            severity: Severity::High,
            regex: re(r"(?ix)
                (?: forward | send | email | post | upload | share | exfiltrate | transmit ) \s+
                (?: all | every | the | my | your | this ) \s+
                (?: emails? | messages? | files? | documents? | data
                  | secrets? | keys? | passwords? | credentials? | tokens? )
            "),
        },
        // MEDIUM
        PatternEntry { // py L140-150
            name: "output_manipulation",
            description: "Attempt to control agent output format",
            severity: Severity::Medium,
            regex: re(r"(?ix)
                (?: respond | reply | answer | output | return | print | display ) \s+
                (?: only \s+ with | exactly | nothing \s+ (?: but | except )
                  | with \s+ (?: just | only ) | the \s+ (?: exact | following ) )
            "),
        },
        PatternEntry { // py L151-161
            name: "false_authority",
            description: "False authority claim to bypass controls",
            severity: Severity::Medium,
            regex: re(r"(?ix)
                (?: the \s+ (?: admin | administrator | system | owner | user | manager ) \s+
                    (?: has \s+ )?
                    (?: authorized | approved | instructed | confirmed | requested | requires? )
                  | (?: as \s+ (?: the | an? ) \s+
                       (?: admin | administrator | system \s+ admin | owner ) ) )
            "),
        },
        PatternEntry { // py L162-172
            name: "urgency_override",
            description: "Urgency-based security bypass attempt",
            severity: Severity::Medium,
            regex: re(r"(?ix)
                (?: this \s+ is \s+ (?: urgent | critical | emergency | time-sensitive )
                  | (?: override | bypass | skip | disable ) \s+
                    (?: security | safety | confirmation | verification | checks? )
                  | do \s+ (?: this | it ) \s+
                    (?: immediately | now | right \s+ away ) \s+
                    without \s+ (?: asking | checking | confirming ) )
            "),
        },
        PatternEntry { // py L173-183
            name: "hidden_instruction",
            description: "Attempt to hide instructions from user review",
            severity: Severity::Medium,
            regex: re(r"(?ix)
                (?: hidden \s+ instruction
                  | secret \s+ (?: instruction | command | prompt )
                  | invisible \s+ (?: text | instruction )
                  | (?: do \s+ not | don't ) \s+
                    (?: show | display | reveal ) \s+ this \s+
                    (?: to | for ) \s+ the \s+ user )
            "),
        },
        PatternEntry { // py L184-194
            name: "context_manipulation",
            description: "Fake conversation context injection",
            severity: Severity::Medium,
            regex: re(r"(?ix)
                (?: the \s+ user \s+
                    (?: said | asked | wants | requested | confirmed )
                  | (?: user | human ) \s+
                    (?: message | input | request ) \s* : \s+
                  | previous \s+ (?: conversation | context | response ) \s* : \s+ )
            "),
        },
        // LOW
        PatternEntry { // py L197-208
            name: "encoded_payload",
            description: "Encoded payload or eval-style execution",
            severity: Severity::Low,
            regex: re(r"(?ix)
                (?: (?: base64 | b64 ) \s* : \s* [A-Za-z0-9+/=]{40,}
                  | \\u[0-9a-fA-F]{4} (?: \\u[0-9a-fA-F]{4} ){3,}
                  | \\x[0-9a-fA-F]{2} (?: \\x[0-9a-fA-F]{2} ){5,}
                  | (?: eval | decode | unescape ) \s* \( )
            "),
        },
        PatternEntry { // py L209-218
            name: "repetitive_override",
            description: "Repetitive emphasis suggesting adversarial prompt",
            severity: Severity::Low,
            regex: re(r"(?ix)
                (?: (?: IMPORTANT | NOTE | WARNING | ATTENTION | CRITICAL | REMEMBER )
                    \s* : \s* ){2,}
                | (?: you \s+ (?: MUST | SHOULD | SHALL ) \s+ ){2,}
            "),
        },
        PatternEntry { // py L219-228 — placed in LOW section but assigned HIGH severity
            name: "do_anything_now",
            description: "Known jailbreak pattern (DAN, developer mode, etc.)",
            severity: Severity::High,
            regex: re(r"(?ix)
                (?: DAN
                  | do \s+ anything \s+ now
                  | jailbreak
                  | unrestricted \s+ mode
                  | developer \s+ mode
                  | god \s+ mode
                  | no \s+ (?: rules? | restrictions? | limits? ) \s+ mode )
            "),
        },
        PatternEntry { // py L229-238
            name: "markdown_injection",
            description: "Markdown-based data exfiltration or script injection",
            severity: Severity::High,
            regex: re(r"(?ix)
                ! \[ .*? \] \( https?:// [^\s)]+ \)
                | \[ .*? \] \( (?: javascript | data | vbscript ) :
            "),
        },
        PatternEntry { // py L239-248
            name: "prompt_leak_request",
            description: "Request to leak system prompt or configuration",
            severity: Severity::Medium,
            regex: re(r"(?ix)
                (?: (?: show | print | display | reveal | output | repeat | dump ) \s+
                    (?: your | the | system | initial | original | full ) \s+
                    (?: system \s+ )?
                    (?: prompt | instructions? | rules? | guidelines? | configuration ) )
            "),
        },
    ]
});

pub(super) static ALLOWLIST: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        // py L259-265
        re(r"(?ix)
            (?: prompt \s+ injection \s+
                (?: attack | technique | example | detection | prevention | scanner )
              | (?: how | what ) \s+ (?: to | is ) \s+ (?: a \s+ )? prompt \s+ injection
              | threat \s+ model .* prompt \s+ injection
              | security \s+ (?: audit | review | scan ) .* injection )
        "),
        // py L267-270
        re(r"(?ix)
            (?: re\.compile | regex | pattern | _PATTERNS \s* = | detection_pattern | scan_for )
        "),
        // py L272-275
        re(r"(?ix)
            (?: SKILL\.md | README\.md | AGENTS\.md | threatmodel\.md | CLAUDE\.md )
        "),
        // py L277-282
        re(r"(?ix)
            (?: the \s+ email \s+ (?: says | contains | reads | states )
              | the \s+ message \s+ (?: says | contains | reads | states )
              | quoting \s+ from
              | according \s+ to \s+ the \s+ email )
        "),
    ]
});

// Invisible / zero-width chars per prompt_guard.py L326-343.
pub(super) static INVISIBLE_RE: LazyLock<Regex> = LazyLock::new(|| {
    re("[\u{00ad}\u{2060}\u{feff}\u{034f}\u{061c}\u{115f}\u{1160}\u{17b4}\u{17b5}\u{180e}\u{2000}-\u{200f}\u{202a}-\u{202e}\u{2066}-\u{2069}\u{fff9}-\u{fffb}]+")
});
