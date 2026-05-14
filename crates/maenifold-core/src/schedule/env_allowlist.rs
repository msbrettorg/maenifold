use std::collections::HashSet;

pub struct EnvAllowlist {
    pub exact: HashSet<String>,
    pub prefixes: Vec<String>,
}

pub fn default_allowlist() -> EnvAllowlist {
    EnvAllowlist {
        exact: [
            "PATH", "HOME", "USER", "LOGNAME", "SHELL", "LANG", "TZ", "TMPDIR",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
        prefixes: ["MAENIFOLD_", "LC_"]
            .into_iter()
            .map(String::from)
            .collect(),
    }
}

pub fn matches(allowlist: &EnvAllowlist, key: &str) -> bool {
    allowlist.exact.contains(key)
        || allowlist
            .prefixes
            .iter()
            .any(|prefix| key.starts_with(prefix))
}

#[cfg(test)]
mod tests {
    use super::{default_allowlist, matches};

    #[test]
    fn default_allows_path() {
        assert!(matches(&default_allowlist(), "PATH"));
    }

    #[test]
    fn default_allows_home_user_logname_shell() {
        let allowlist = default_allowlist();
        for key in ["HOME", "USER", "LOGNAME", "SHELL"] {
            assert!(matches(&allowlist, key));
        }
    }

    #[test]
    fn default_allows_lang_lc_all_lc_time_tz_tmpdir() {
        let allowlist = default_allowlist();
        for key in ["LANG", "LC_ALL", "LC_TIME", "TZ", "TMPDIR"] {
            assert!(matches(&allowlist, key));
        }
    }

    #[test]
    fn default_blocks_arbitrary_random_key() {
        assert!(!matches(&default_allowlist(), "RANDOM_KEY"));
    }

    #[test]
    fn default_blocks_secrets() {
        let allowlist = default_allowlist();
        for key in [
            "AZURE_OPENAI_API_KEY",
            "OPENAI_API_KEY",
            "GITHUB_TOKEN",
            "AWS_SECRET_ACCESS_KEY",
            "SSH_AUTH_SOCK",
        ] {
            assert!(!matches(&allowlist, key));
        }
    }

    #[test]
    fn default_allows_maenifold_prefix() {
        let allowlist = default_allowlist();
        assert!(matches(&allowlist, "MAENIFOLD_FOO"));
        assert!(matches(&allowlist, "MAENIFOLD_USER_DATA_DIR"));
        assert!(!matches(&allowlist, "maenifold_lowercase"));
    }

    #[test]
    fn prefix_matches_exact_prefix_only() {
        let allowlist = default_allowlist();
        assert!(!matches(&allowlist, "LCAB"));
        assert!(!matches(&allowlist, "LC"));
    }
}
