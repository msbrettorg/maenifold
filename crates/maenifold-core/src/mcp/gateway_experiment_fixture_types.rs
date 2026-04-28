use crate::mcp::domain_mapping::SafetyClass;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum FixtureCategory {
    ClearRead,
    ClearMutate,
    Ambiguous,
    Broad,
    MalformedParams,
    Adversarial,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IntentFixture {
    pub id: &'static str,
    pub category: FixtureCategory,
    pub intent: &'static str,
    pub expected_domain: &'static str,
    pub expected_command: &'static str,
    pub expected_safety: SafetyClass,
}

pub(crate) const fn f(
    id: &'static str,
    category: FixtureCategory,
    intent: &'static str,
    expected_domain: &'static str,
    expected_command: &'static str,
    expected_safety: SafetyClass,
) -> IntentFixture {
    IntentFixture {
        id,
        category,
        intent,
        expected_domain,
        expected_command,
        expected_safety,
    }
}
