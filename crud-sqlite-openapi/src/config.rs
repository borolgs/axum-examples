use std::sync::OnceLock;

use serde::Deserialize;

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RegistrationMode {
    Free,
    WhiteList,
    WaitList,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_database_url")]
    pub database_url: String,

    // build
    pub app_version: Option<String>,
    #[serde(default = "default_local")]
    pub source: String,
    #[serde(default = "default_local")]
    pub git_commit: String,
    #[serde(default = "default_local")]
    pub pipeline_id: String,
    #[serde(default = "default_local")]
    pub version: String,
}

fn default_port() -> u16 {
    4000
}

fn default_database_url() -> String {
    "sqlite.db".into()
}

fn default_local() -> String {
    "local".into()
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();
        let config = envy::from_env::<Self>().unwrap();

        config
    }
}

static CONFIG: OnceLock<Config> = OnceLock::new();

pub fn config() -> &'static Config {
    CONFIG.get_or_init(|| Config::from_env())
}

#[cfg(test)]
pub fn config_override<F>(override_config: F) -> &'static Config
where
    F: FnOnce(Config) -> Config,
{
    CONFIG.get_or_init(|| override_config(Config::from_env()))
}
