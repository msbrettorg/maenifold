use crate::error::GateError;
use crate::gate_definition::{GateDefinition, GateState, TransitionResult};

#[derive(Debug, Clone)]
pub struct StateMachine;

impl StateMachine {
    pub fn acting_stage_name(
        state: &str,
        definition: &GateDefinition,
    ) -> Result<String, GateError> {
        let (acting_stage, _) = Self::acting_state(state, definition)?;
        Ok(acting_stage)
    }

    pub fn next_on_pass(
        state: &str,
        definition: &GateDefinition,
    ) -> Result<TransitionResult, GateError> {
        Self::transition(state, "pass", definition)
    }

    pub fn next_on_fail(
        state: &str,
        definition: &GateDefinition,
    ) -> Result<TransitionResult, GateError> {
        Self::transition(state, "fail", definition)
    }

    fn transition(
        state: &str,
        outcome: &str,
        definition: &GateDefinition,
    ) -> Result<TransitionResult, GateError> {
        let (acting_stage, gate_state) = Self::acting_state(state, definition)?;
        let next_state =
            gate_state
                .on
                .get(outcome)
                .cloned()
                .ok_or_else(|| GateError::IllegalTransition {
                    state: state.to_string(),
                    outcome: outcome.to_string(),
                })?;

        Ok(TransitionResult {
            acting_stage,
            outcome: outcome.to_string(),
            next_state,
        })
    }

    fn acting_state<'a>(
        state: &str,
        definition: &'a GateDefinition,
    ) -> Result<(String, &'a GateState), GateError> {
        let initial = definition.initial_state()?;
        let normalized_state = if state.is_empty() { initial } else { state };
        let gate_state = definition
            .find_state(normalized_state)
            .ok_or_else(|| GateError::UnknownState(normalized_state.to_string()))?;

        if gate_state.terminal {
            return Err(GateError::IllegalTransition {
                state: normalized_state.to_string(),
                outcome: "terminal".to_string(),
            });
        }

        Ok((normalized_state.to_string(), gate_state))
    }
}

#[cfg(test)]
mod tests {
    use super::StateMachine;
    use crate::gate_definition::GateDefinition;

    fn sdd_definition() -> GateDefinition {
        GateDefinition::from_json_text(
            r#"{
              "schemaVersion": 1,
              "id": "sdd",
              "name": "Spec Driven Development",
              "initial": "pending",
              "states": {
                "pending": { "on": { "pass": "swe" } },
                "swe": { "on": { "fail": "pending", "pass": "red-team" } },
                "red-team": { "on": { "fail": "pending", "pass": "blue-team" } },
                "blue-team": { "on": { "fail": "pending", "pass": "done" } },
                "done": { "terminal": true }
              }
            }"#,
        )
        .expect("valid definition")
    }

    #[test]
    fn pending_pass_advances_exactly_one_stage() {
        let transition =
            StateMachine::next_on_pass("pending", &sdd_definition()).expect("pending pass");

        assert_eq!(transition.acting_stage, "pending");
        assert_eq!(transition.next_state, "swe");
    }

    #[test]
    fn pending_append_uses_pending_stage() {
        let acting_stage =
            StateMachine::acting_stage_name("pending", &sdd_definition()).expect("pending stage");

        assert_eq!(acting_stage, "pending");
    }

    #[test]
    fn swe_fail_returns_to_pending() {
        let t = StateMachine::next_on_fail("swe", &sdd_definition()).expect("swe fail");
        assert_eq!(t.acting_stage, "swe");
        assert_eq!(t.next_state, "pending");
    }

    #[test]
    fn swe_pass_advances_to_red_team() {
        let t = StateMachine::next_on_pass("swe", &sdd_definition()).expect("swe pass");
        assert_eq!(t.acting_stage, "swe");
        assert_eq!(t.next_state, "red-team");
    }

    #[test]
    fn red_team_pass_advances_to_blue_team() {
        let t = StateMachine::next_on_pass("red-team", &sdd_definition()).expect("red pass");
        assert_eq!(t.acting_stage, "red-team");
        assert_eq!(t.next_state, "blue-team");
    }

    #[test]
    fn blue_team_pass_reaches_done() {
        let t = StateMachine::next_on_pass("blue-team", &sdd_definition()).expect("blue pass");
        assert_eq!(t.acting_stage, "blue-team");
        assert_eq!(t.next_state, "done");
    }

    #[test]
    fn empty_state_defaults_to_initial() {
        let stage = StateMachine::acting_stage_name("", &sdd_definition()).expect("empty state");
        assert_eq!(stage, "pending");
    }

    #[test]
    fn unknown_state_returns_error() {
        let result = StateMachine::next_on_pass("nonexistent", &sdd_definition());
        assert!(result.is_err());
    }

    #[test]
    fn terminal_state_rejects_transition() {
        let result = StateMachine::next_on_pass("done", &sdd_definition());
        assert!(result.is_err());
    }

    #[test]
    fn pending_has_no_fail_transition() {
        let result = StateMachine::next_on_fail("pending", &sdd_definition());
        assert!(result.is_err());
    }
}
