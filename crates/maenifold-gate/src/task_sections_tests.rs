    use super::TaskSectionParser;

    #[test]
    fn non_gate_section_produces_no_records() {
        let lines = vec![
            "## Acceptance Criteria",
            "",
            "1. valid login returns token",
            "2. invalid login returns 401",
        ];
        let records = TaskSectionParser::parse(&lines).expect("parse should succeed");
        assert!(records.is_empty());
    }

    #[test]
    fn missing_required_field_returns_error() {
        let lines = vec![
            "## SWE",
            "",
            "- Stage ID: swe",
            "- Record Kind: outcome",
            "- Recorded At: 2026-01-01T00:00:00Z",
            "- Notes: implemented",
        ];
        let result = TaskSectionParser::parse(&lines);
        assert!(result.is_err(), "missing Agent should cause an error");
    }

    #[test]
    fn multiple_sections_parse_independently() {
        let lines = vec![
            "## Pending",
            "",
            "- Stage ID: pending",
            "- Record Kind: outcome",
            "- Agent: pm",
            "- Recorded At: 2026-01-01T00:00:00Z",
            "- Outcome: PASS",
            "- Notes: spec approved",
            "",
            "## SWE",
            "",
            "- Stage ID: swe",
            "- Record Kind: append",
            "- Agent: swe-agent",
            "- Recorded At: 2026-01-02T00:00:00Z",
            "- Notes: implementation started",
        ];
        let records = TaskSectionParser::parse(&lines).expect("parse should succeed");
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].stage, "pending");
        assert_eq!(records[0].outcome.as_deref(), Some("pass"));
        assert_eq!(records[1].stage, "swe");
        assert_eq!(records[1].record_kind, "append");
        assert_eq!(records[1].outcome, None);
    }

    #[test]
    fn unknown_fields_are_silently_ignored() {
        let lines = vec![
            "## SWE",
            "",
            "- Stage ID: swe",
            "- Record Kind: outcome",
            "- Agent: swe-agent",
            "- Recorded At: 2026-01-01T00:00:00Z",
            "- Outcome: PASS",
            "- Custom: extra field",
            "- Notes: done",
        ];
        let records = TaskSectionParser::parse(&lines).expect("parse should succeed");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].agent, "swe-agent");
        assert_eq!(records[0].notes, "done");
    }

    #[test]
    fn mixed_gate_and_non_gate_sections() {
        let lines = vec![
            "## Acceptance Criteria",
            "",
            "1. valid login returns token",
            "",
            "## Pending",
            "",
            "- Stage ID: pending",
            "- Record Kind: outcome",
            "- Agent: pm",
            "- Recorded At: 2026-01-01T00:00:00Z",
            "- Outcome: PASS",
            "- Notes: spec approved",
        ];
        let records = TaskSectionParser::parse(&lines).expect("parse should succeed");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].stage, "pending");
    }

    /// Regression test for T-202.8: H2 headings inside fenced code blocks should
    /// be ignored by the parser. Prior to fix, the parser would treat any line
    /// starting with `## ` as a section boundary, causing spurious "gate section
    /// is missing Stage ID" errors when code examples contained H2 headings with
    /// gate-like fields beneath them.
    #[test]
    fn h2_inside_fenced_code_block_is_ignored() {
        // This example has a code block containing both an H2 heading AND a gate-like
        // field "- Agent: ..." which triggers the bug: the parser sees a gate field
        // without a Stage ID and errors.
        let lines = vec![
            "## Acceptance Criteria",
            "",
            "1. Parser should ignore H2 inside code blocks",
            "",
            "## Design",
            "",
            "Here is an example of a gate section in markdown:",
            "",
            "```markdown",
            "## SWE",
            "",
            "- Stage ID: swe",
            "- Record Kind: outcome",
            "- Agent: swe-agent",
            "- Recorded At: 2026-01-01T00:00:00Z",
            "- Outcome: PASS",
            "- Notes: implementation complete",
            "```",
            "",
            "## Notes",
            "",
            "Real notes here.",
        ];

        // The parse should succeed — currently fails with "gate section is missing Stage ID"
        // because the parser incorrectly treats "## SWE" inside the code block as a real
        // section boundary, then sees gate fields but with values treated as the wrong section.
        let records = TaskSectionParser::parse(&lines).expect(
            "parse should succeed: H2 inside fenced code block must be ignored",
        );

        // Should produce zero gate records (none of the REAL sections have gate fields)
        assert!(
            records.is_empty(),
            "expected no gate records, but got {} — parser may have incorrectly \
             parsed the code block example as a real gate section",
            records.len()
        );
    }

    /// Variant: H2 inside fenced code block followed by a real gate section.
    /// Ensures the parser both ignores fenced H2 AND still parses real sections.
    #[test]
    fn fenced_code_block_does_not_break_subsequent_gate_section() {
        let lines = vec![
            "## Design",
            "",
            "Example markdown:",
            "",
            "```markdown",
            "## Fake Heading In Code",
            "- Agent: not-real",
            "```",
            "",
            "## Pending",
            "",
            "- Stage ID: pending",
            "- Record Kind: outcome",
            "- Agent: pm",
            "- Recorded At: 2026-01-01T00:00:00Z",
            "- Outcome: PASS",
            "- Notes: spec approved",
        ];

        let records = TaskSectionParser::parse(&lines).expect(
            "parse should succeed even with H2 inside fenced code block",
        );

        // Should have exactly one record from the real "## Pending" section
        assert_eq!(
            records.len(),
            1,
            "expected 1 gate record from ## Pending, got {}",
            records.len()
        );
        assert_eq!(records[0].stage, "pending");
        assert_eq!(records[0].agent, "pm");
    }
