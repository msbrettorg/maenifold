use super::{
    CopilotModel, CopilotTokenStoreError, GetCopilotModelsError,
    client::{CopilotEndpointResolverError, CopilotModelsClientError},
};

pub(super) fn filter_models(models: Vec<CopilotModel>) -> Vec<CopilotModel> {
    models
        .into_iter()
        .filter(|model| model.model_picker_enabled)
        .filter(|model| model.capabilities.model_type.eq_ignore_ascii_case("chat"))
        .filter(|model| {
            model
                .policy
                .as_ref()
                .and_then(|policy| policy.state.as_deref())
                .is_some_and(|state| state.eq_ignore_ascii_case("enabled"))
        })
        .collect()
}

pub(super) fn map_token_error(error: CopilotTokenStoreError) -> GetCopilotModelsError {
    match error {
        CopilotTokenStoreError::TokenMissing { .. }
        | CopilotTokenStoreError::HostsJsonMissing { .. } => GetCopilotModelsError::TokenMissing,
        _ => GetCopilotModelsError::Other,
    }
}

pub(super) fn map_client_error(error: &CopilotModelsClientError) -> GetCopilotModelsError {
    match error {
        CopilotModelsClientError::Forbidden => GetCopilotModelsError::Forbidden,
        CopilotModelsClientError::Parse(_) => GetCopilotModelsError::ParseFailed,
        CopilotModelsClientError::Network(_)
        | CopilotModelsClientError::Timeout(_)
        | CopilotModelsClientError::Endpoint(CopilotEndpointResolverError::Network(_))
        | CopilotModelsClientError::Endpoint(CopilotEndpointResolverError::Timeout(_)) => {
            GetCopilotModelsError::EndpointUnreachable
        }
        _ => GetCopilotModelsError::Other,
    }
}
