use chrono::{DateTime, Utc};

pub struct TokenInfo {
    pub access_token: String,
    pub expires_at: DateTime<Utc>,
    pub refresh_token: Option<String>,
}
