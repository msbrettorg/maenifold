use crate::error::{MaenifoldError, Result};

pub(crate) fn require_adopted_role(
    invocation: &str,
    requires_role: Option<&str>,
    adopt_role: Option<&str>,
) -> Result<()> {
    let required_role = requires_role
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let adopted_role = adopt_role.map(str::trim).filter(|value| !value.is_empty());

    let Some(required_role) = required_role else {
        return Ok(());
    };

    let Some(adopted_role) = adopted_role else {
        return Err(MaenifoldError::Message(format!(
            "ERROR: {invocation} requires adopt.role '{required_role}', but no role adoption is active"
        )));
    };

    if adopted_role != required_role {
        return Err(MaenifoldError::Message(format!(
            "ERROR: {invocation} requires adopt.role '{required_role}', but active role is '{adopted_role}'"
        )));
    }

    Ok(())
}
