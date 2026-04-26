use serde::{Deserialize, Serialize};

fn default_mode() -> String {
    "endophasia".to_string()
}

fn default_prompt() -> String {
    "Generate an image of what's top of mind.".to_string()
}

fn default_size() -> String {
    "1536x1024".to_string()
}

fn default_quality() -> String {
    "low".to_string()
}

fn default_input_fidelity() -> String {
    "medium".to_string()
}

fn default_max_retries() -> u32 {
    3
}

fn default_community_count() -> u32 {
    10
}

fn default_rotate_minutes() -> u32 {
    5
}

fn default_endpoint_env_var() -> String {
    "AZURE_OPENAI_ENDPOINT".to_string()
}

fn default_api_key_env_var() -> String {
    "AZURE_OPENAI_API_KEY".to_string()
}

fn default_image_model() -> String {
    "gpt-image-2".to_string()
}

fn default_small_model() -> String {
    "gpt-5.4-mini".to_string()
}

fn default_reasoning_model() -> String {
    "gpt-5.4".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct EasterEggsSettings {
    #[serde(default)]
    pub wallpaper: WallpaperSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WallpaperSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default = "default_prompt")]
    pub prompt: String,
    #[serde(default)]
    pub azure: WallpaperAzureSettings,
    #[serde(default = "default_size")]
    pub size: String,
    #[serde(default = "default_quality")]
    pub quality: String,
    #[serde(default)]
    pub continuity: bool,
    #[serde(default = "default_input_fidelity")]
    pub input_fidelity: String,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    #[serde(default = "default_community_count")]
    pub community_count: u32,
    #[serde(default = "default_rotate_minutes")]
    pub rotate_minutes: u32,
}

impl Default for WallpaperSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: default_mode(),
            prompt: default_prompt(),
            azure: WallpaperAzureSettings::default(),
            size: default_size(),
            quality: default_quality(),
            continuity: false,
            input_fidelity: default_input_fidelity(),
            max_retries: default_max_retries(),
            community_count: default_community_count(),
            rotate_minutes: default_rotate_minutes(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WallpaperAzureSettings {
    #[serde(default = "default_endpoint_env_var")]
    pub endpoint_env_var: String,
    #[serde(default = "default_api_key_env_var")]
    pub api_key_env_var: String,
    #[serde(default = "default_image_model")]
    pub image_model: String,
    #[serde(default = "default_small_model")]
    pub small_model: String,
    #[serde(default = "default_reasoning_model")]
    pub reasoning_model: String,
}

impl Default for WallpaperAzureSettings {
    fn default() -> Self {
        Self {
            endpoint_env_var: default_endpoint_env_var(),
            api_key_env_var: default_api_key_env_var(),
            image_model: default_image_model(),
            small_model: default_small_model(),
            reasoning_model: default_reasoning_model(),
        }
    }
}
