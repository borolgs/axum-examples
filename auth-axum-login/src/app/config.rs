use std::sync::OnceLock;

use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    #[serde(default = "default_database_url")]
    pub database_url: String,

    #[serde(rename = "github_oauth_redirect_url")]
    pub github_redirect_url: String,
    #[serde(rename = "github_oauth_client_id")]
    pub github_client_id: String,
    #[serde(rename = "github_oauth_client_secret")]
    pub github_client_secret: String,

    #[serde(skip)]
    #[cfg(test)]
    pub skip_auth: bool,
}

fn default_database_url() -> String {
    "sqlite.db".into()
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
pub fn config_override<F>(config: F) -> &'static Config
where
    F: FnOnce(Config) -> Config,
{
    CONFIG.get_or_init(|| config(Config::from_env()))
}
