#![allow(clippy::module_name_repetitions)]

use serde::Deserialize;

#[allow(dead_code)]
pub const LAST_FM_API_KEY: &str = "732433605ea7893c761d340a05752695";
#[allow(dead_code)]
pub const LAST_FM_SHARED_SECRET: &str = "420fdb301e6b4a62a888bf51def71670";
pub const FANART_PROJECT_KEY: &str = "df5eb171c6e0e49122ad59830cdf789f";

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
