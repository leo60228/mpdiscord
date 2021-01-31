use super::config::Config;
use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct AccountSource {
    pub note: String,
}

#[derive(Deserialize)]
pub struct Account {
    pub acct: String,
    pub source: AccountSource,
}

pub struct Mastodon {
    client: Client,
    token: String,
}

impl Mastodon {
    pub fn new(config: &Config) -> Self {
        let client = Client::new();
        let token = config.mastodon_token.clone();
        Self { client, token }
    }

    pub async fn account(&self) -> Result<Account> {
        let account = self
            .client
            .get("https://60228.dev/api/v1/accounts/verify_credentials")
            .bearer_auth(&self.token)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        Ok(account)
    }

    pub async fn set_bio(&self, bio: &str) -> Result<()> {
        self.client
            .patch("https://60228.dev/api/v1/accounts/update_credentials")
            .bearer_auth(&self.token)
            .form(&[("note", bio)])
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}
