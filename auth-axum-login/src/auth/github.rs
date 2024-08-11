use oauth2::{basic::BasicClient, AuthUrl, ClientId, ClientSecret, CsrfToken, RedirectUrl, TokenUrl};
use reqwest::Client;
use serde::Deserialize;

use crate::config::{config, Config};

#[derive(Debug, Deserialize)]
pub struct GithubOAuthResponse {
    pub code: String,
    pub state: CsrfToken,
}

#[derive(Debug, Deserialize)]
pub struct GithubUserResponse {
    pub login: String,
    pub email: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GithubEmailResponse {
    pub email: String,
    pub primary: bool,
}

pub fn create_github_oauth_client() -> BasicClient {
    let config = config();

    BasicClient::new(
        ClientId::new(config.github_client_id.clone()),
        Some(ClientSecret::new(config.github_client_secret.clone())),
        AuthUrl::new("https://github.com/login/oauth/authorize".into()).unwrap(),
        Some(TokenUrl::new("https://github.com/login/oauth/access_token".into()).unwrap()),
    )
    .set_redirect_uri(RedirectUrl::new(config.github_redirect_url.clone()).unwrap())
}

pub async fn get_github_user_email(client: &Client, access_token: &str) -> reqwest::Result<Option<String>> {
    let mut user_email = client
        .get("https://api.github.com/user")
        .bearer_auth(&access_token)
        .send()
        .await?
        .json::<GithubUserResponse>()
        .await
        .map(|e| e.email)?;

    if user_email.is_none() {
        user_email = client
            .get("https://api.github.com/user/emails")
            .bearer_auth(&access_token)
            .send()
            .await?
            .json::<Vec<GithubEmailResponse>>()
            .await?
            .into_iter()
            .find(|email| email.primary)
            .map(|e| e.email);
    }

    Ok(user_email)
}
