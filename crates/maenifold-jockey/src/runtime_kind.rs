use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeKind {
    Mock,
    ClaudeCode,
    GeminiCli,
    CodexCli,
}

impl RuntimeKind {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "mock" => Some(Self::Mock),
            "claude" | "claude-code" | "claude-acp" => Some(Self::ClaudeCode),
            "gemini" | "gemini-cli" => Some(Self::GeminiCli),
            "codex" | "codex-cli" | "codex-acp" => Some(Self::CodexCli),
            _ => None,
        }
    }

    pub fn runtime_key(self) -> &'static str {
        match self {
            Self::Mock => "mock",
            Self::ClaudeCode => "claude-code",
            Self::GeminiCli => "gemini-cli",
            Self::CodexCli => "codex-cli",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Mock => "Mock",
            Self::ClaudeCode => "Claude Code",
            Self::GeminiCli => "Gemini CLI",
            Self::CodexCli => "Codex CLI",
        }
    }

    pub fn is_mock(self) -> bool {
        matches!(self, Self::Mock)
    }

    pub fn install_hint(self) -> &'static str {
        match self {
            Self::ClaudeCode => "npm install -g @anthropic-ai/claude-code",
            Self::GeminiCli => "npm install -g @google/gemini-cli",
            Self::CodexCli => "npm install -g @openai/codex",
            Self::Mock => "",
        }
    }
}
