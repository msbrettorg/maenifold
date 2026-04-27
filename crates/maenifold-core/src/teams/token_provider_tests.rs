use super::*;
use crate::teams::credential_store::agency_cowork_token_cache_path_from_home_vars;

#[test]
fn teams_token_classifies_aadsts530084_and_blocks_cli_for_process_lifetime() {
    reset_token_protection_block_for_tests();

    let error = classify_azure_cli_error("AADSTS530084: Proof of possession is required");

    assert_eq!(
        error,
        TeamsTokenError::TokenProtectionRequiresBrowserSession
    );
}

#[test]
fn teams_token_cache_path_errors_when_home_directory_is_unavailable() {
    let error = agency_cowork_token_cache_path_from_home_vars(None, None)
        .expect_err("missing home directory must be explicit");
    let message = error.to_string();

    assert!(message.contains("HOME/USERPROFILE"));
    assert!(message.contains("~/.teams-agent/token-cache.json"));
}

#[test]
fn teams_token_error_codes_are_stable_for_ui_status() {
    assert_eq!(
        TeamsTokenError::TokenProtectionRequiresBrowserSession.code(),
        "TokenProtectionRequiresBrowserSession"
    );
    assert_eq!(
        TeamsTokenError::AzureCliUnavailable.code(),
        "AzureCliUnavailable"
    );
    assert!(
        TeamsTokenError::AzureCliUnavailable
            .to_string()
            .contains("~/.teams-agent/token-cache.json")
    );
    assert_eq!(
        TeamsTokenError::AzureCliNotLoggedIn.code(),
        "AzureCliNotLoggedIn"
    );
}

#[test]
fn teams_token_prefers_homebrew_azure_cli_locations() {
    assert_eq!(
        azure_cli_binary_from_candidates(|path| path == "/opt/homebrew/bin/az"),
        "/opt/homebrew/bin/az"
    );
    assert_eq!(
        azure_cli_binary_from_candidates(|path| path == "/usr/local/bin/az"),
        "/usr/local/bin/az"
    );
    assert_eq!(azure_cli_binary_from_candidates(|_| false), "az");
}

#[tokio::test]
#[ignore = "requires local Azure CLI sign-in or signed-in Teams browser cache"]
async fn teams_token_live_provider_is_runnable_locally() {
    let provider = TeamsTokenProvider::new();
    let acquisition = provider.acquire_token(false).await;
    if let Ok(acquisition) = acquisition {
        assert!(!acquisition.token.access_token.is_empty());
    }
}
