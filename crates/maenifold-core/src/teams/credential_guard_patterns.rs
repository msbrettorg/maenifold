//! Credential pattern table — port of agency-cowork
//! `skills/teams/scripts/rich/credential_scanner.py` `_PATTERNS` (L32-162)
//! and `_ALLOWLIST` (L171-187).
//!
//! Rust `regex` crate has no lookbehind/lookahead, so the git-SHA allowlist
//! uses `\b` word boundaries instead of `(?<![A-Za-z0-9+/])` — accepted as
//! a minor fidelity loss for v1.

use std::sync::LazyLock;

use regex::Regex;

pub(super) struct PatternEntry {
    pub name: &'static str,
    pub description: &'static str,
    pub regex: Regex,
}

fn re(pattern: &str) -> Regex {
    Regex::new(pattern).expect("credential_guard pattern must compile")
}

pub(super) static PATTERNS: LazyLock<Vec<PatternEntry>> = LazyLock::new(|| {
    vec![
        PatternEntry { // py L34-41
            name: "private_key",
            description: "PEM-encoded private key header",
            regex: re(r"(?i)-----BEGIN\s+(?:RSA\s+|EC\s+|OPENSSH\s+|DSA\s+|ENCRYPTED\s+)?PRIVATE\s+KEY-----"),
        },
        PatternEntry { // py L43-49
            name: "jwt_token",
            description: "JSON Web Token (JWT)",
            regex: re(r"eyJ[A-Za-z0-9_-]{10,}\.eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}"),
        },
        PatternEntry { // py L51-59
            name: "azure_sas_token",
            description: "Azure SAS token or Shared Access Signature",
            regex: re(r"(?i)(?:sv=\d{4}-\d{2}-\d{2}&.*?sig=[A-Za-z0-9%+/=]{20,})|(?:SharedAccessSignature\s+sr=[^\s]{10,})"),
        },
        PatternEntry { // py L61-65
            name: "azure_storage_key",
            description: "Azure Storage account key",
            regex: re(r"(?:AccountKey|account_key)\s*=\s*[A-Za-z0-9+/]{86}=="),
        },
        PatternEntry { // py L67-71
            name: "aws_access_key",
            description: "AWS access key ID",
            regex: re(r"AKIA[0-9A-Z]{16}"),
        },
        PatternEntry { // py L73-80
            name: "aws_secret_key",
            description: "AWS secret access key",
            regex: re(r"(?i)(?:aws_secret_access_key|secret_access_key|aws_secret)\s*[:=]\s*[A-Za-z0-9+/]{40}"),
        },
        PatternEntry { // py L82-86
            name: "github_token",
            description: "GitHub personal access / OAuth / app token",
            regex: re(r"gh[pousr]_[A-Za-z0-9_]{36,}"),
        },
        PatternEntry { // py L88-95
            name: "azure_client_secret",
            description: "Azure AD / Entra client secret",
            regex: re(r#"(?i)(?:client_secret|clientSecret|AZURE_CLIENT_SECRET)\s*[:=]\s*[^\s"',]{20,}"#),
        },
        PatternEntry { // py L98-105
            name: "generic_api_key",
            description: "Generic API key assignment",
            regex: re(r#"(?i)(?:api[_-]?key|apikey|x-api-key|api_secret)\s*(?:[:=]|->)\s*["']?[A-Za-z0-9_\-+/]{20,}["']?"#),
        },
        PatternEntry { // py L109-116
            name: "bearer_token",
            description: "Bearer authorization token",
            regex: re(r"(?i)(?:Authorization|Bearer)\s*[:=]\s*(?:Bearer\s*)?[A-Za-z0-9_\-.]{40,}"),
        },
        PatternEntry { // py L118-131
            name: "connection_string",
            description: "Database connection string with credentials",
            regex: re(r#"(?i)(?:(?:Server|Data\s+Source)\s*=\s*[^;]+;\s*(?:.*?Password|.*?Pwd)\s*=\s*[^;]+|mongodb(?:\+srv)?://[^\s"']{10,}|postgres(?:ql)?://[^\s"']{10,}|redis://[^\s"']{10,}|mysql://[^\s"']{10,})"#),
        },
        PatternEntry { // py L133-140
            name: "password_assignment",
            description: "Password or secret value assignment",
            regex: re(r#"(?i)(?:password|passwd|pwd|secret|token|credential)\s*[:=]\s*["']?[^\s"',]{8,}["']?"#),
        },
        PatternEntry { // py L142-152
            name: "env_secret",
            description: "Environment variable containing a secret",
            regex: re(r"(?i)(?:AZURE_OPENAI_API_KEY|OPENAI_API_KEY|GITHUB_TOKEN|SLACK_TOKEN|SENDGRID_API_KEY|TWILIO_AUTH_TOKEN|STRIPE_SECRET_KEY|DATABASE_URL|REDIS_URL|MONGO_URI|SECRET_KEY|ENCRYPTION_KEY)\s*=\s*[^\s]{8,}"),
        },
        PatternEntry { // py L154-160
            name: "azure_openai_key",
            description: "Azure OpenAI / Cognitive Services subscription key",
            regex: re(r"(?i)(?:api[_-]?key|ocp-apim-subscription-key)\s*[:=]\s*[0-9a-f]{32}"),
        },
    ]
});

/// Allowlist — known safe patterns. py L171-187.
/// Note: git-SHA pattern (py L180) loses `(?<![A-Za-z0-9+/])` lookbehind /
/// `(?![A-Za-z0-9+/=])` lookahead due to Rust regex limitations; uses `\b`.
pub(super) static ALLOWLIST: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        // py L175-178: CorrelationId UUID
        re(r"(?i)CorrelationId:\s*[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}\b"),
        // py L180: git commit SHA (40 hex chars)
        re(r"\b[0-9a-f]{40}\b"),
        // py L182: UUIDs
        re(r"\b[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}\b"),
        // py L184: Teams meeting thread ID
        re(r"19:meeting_[A-Za-z0-9+/=]+@thread\.v2"),
        // py L185: Teams thread IDs
        re(r"19:[0-9a-f]+@thread\.(?:v2|tacv2)"),
        // py L186: Teams unq.gbl.spaces IDs
        re(r"19:[0-9a-f-]+_[0-9a-f-]+@unq\.gbl\.spaces"),
    ]
});
