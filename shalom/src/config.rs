#![allow(clippy::module_name_repetitions)]

use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    pub home_assistant: HomeAssistantConfig,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct HomeAssistantConfig {
    pub uri: String,
    pub token: String,
}
