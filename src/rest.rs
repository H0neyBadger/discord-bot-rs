use reqwest::{header, Client};
use serde::de::DeserializeOwned;

pub struct Rest {
    client: Client,
    version: u8,
}

impl Rest {
    pub fn new(bot_token: &str) -> Self {
        let mut headers = header::HeaderMap::new();
        let mut auth_value =
            header::HeaderValue::from_str(format!("Bot {}", bot_token).as_str()).unwrap();
        auth_value.set_sensitive(true);
        headers.insert(header::AUTHORIZATION, auth_value);
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/json"),
        );
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();
        Self {
            client: client,
            version: 10,
        }
    }

    pub async fn me<T>(&self) -> Result<T, ()>
    where
        T: DeserializeOwned,
    {
        let res = self
            .client
            .get("https://discord.com/api/oauth2/applications/@me")
            .send()
            .await
            .unwrap()
            .json::<T>()
            .await
            .unwrap();
        Ok(res)
    }

    pub async fn react(&self, channel_id: &str, message_id: &str, emoji: &str) -> Result<(), ()> {
        let url = format!(
            "https://discord.com/api/v{}/channels/{channel_id}/messages/{message_id}/reactions/{emoji}/@me",
            self.version
        );
        let _res = self
            .client
            .put(url)
            .body("")
            .header("content-length", 0)
            .send()
            .await
            .unwrap();
        Ok(())
    }
}
