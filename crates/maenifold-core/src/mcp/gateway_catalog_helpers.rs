use super::{CatalogCommand, CatalogDomain, GatewayCatalog};
use crate::error::{MaenifoldError, Result};
use std::collections::BTreeMap;

impl GatewayCatalog {
    pub(super) fn insert_domain(
        &mut self,
        name: &str,
        description: &str,
    ) -> Result<&mut CatalogDomain> {
        Ok(self
            .domains
            .entry(name.to_string())
            .or_insert_with(|| CatalogDomain {
                description: description.to_string(),
                commands: BTreeMap::new(),
            }))
    }

    pub(super) fn domain(&self, domain: &str) -> Result<&CatalogDomain> {
        self.domains.get(domain).ok_or_else(|| {
            let mut available = self.domains.keys().map(String::as_str).collect::<Vec<_>>();
            available.sort_unstable();
            MaenifoldError::GatewayInvalidParams(format!(
                "Unknown gateway domain '{domain}'. Available domains: {}.",
                available.join(", ")
            ))
        })
    }

    pub(super) fn command_record(&self, domain: &str, command: &str) -> Result<&CatalogCommand> {
        let domain_record = self.domain(domain)?;
        domain_record.commands.get(command).ok_or_else(|| {
            let suggestions = self
                .domains
                .iter()
                .filter(|(name, _)| name.as_str() != domain)
                .filter(|(_, candidate)| candidate.commands.contains_key(command))
                .map(|(name, _)| format!("{name}/{command}"))
                .collect::<Vec<_>>();

            let available = domain_record
                .commands
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>();

            let mut message = format!("Unknown gateway command '{command}' for domain '{domain}'.");
            if !suggestions.is_empty() {
                message.push_str(&format!(" Did you mean: {}?", suggestions.join(", ")));
            }
            if !available.is_empty() {
                message.push_str(&format!(
                    " Available commands in '{domain}': {}.",
                    available.join(", ")
                ));
            }
            MaenifoldError::GatewayInvalidParams(message)
        })
    }
}
