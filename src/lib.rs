//! Implments Rust API to Pixiv.
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]
mod de;
pub mod error;
pub mod types;

use std::str::FromStr;

use regex::Regex;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
};
use serde::de::DeserializeOwned;
use types::WrappedResponse;

pub use crate::error::Error;
pub use crate::types::*;

/// A `Result` alias where the `Err` case is `pixrs::Error`.
pub type Result<T> = std::result::Result<T, crate::Error>;

/// The client to send Pixiv API requests.
pub struct PixivClient {
    client: Client,
    #[allow(dead_code)] // TODO For POST requests
    csrf_token: String,
}

static BASE_URL_HTTPS: &str = "https://www.pixiv.net";
static USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/114.0.0.0 Safari/537.36";

impl PixivClient {
    /// Creates a new client.
    /// ## Argument
    /// * `token`: The session token on your web session. See the [PixivFE guide](https://pixivfe.pages.dev/obtaining-pixivfe-token/) for how to get it.
    pub async fn new(token: &str) -> Result<Self> {
        let cookie = format!("PHPSESSID={token}");
        let mut headers = HeaderMap::new();
        let mut cookie = HeaderValue::from_str(&cookie)
            .map_err(|_| crate::Error::Other("Cookies data seems to be invaild"))?;
        cookie.set_sensitive(true);
        headers.append(reqwest::header::COOKIE, cookie);
        headers.append(
            reqwest::header::REFERER,
            HeaderValue::from_static(BASE_URL_HTTPS),
        );
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .default_headers(headers)
            .build()?;
        let csrf_token = PixivClient::csrf_token(&client).await?;
        Ok(PixivClient { client, csrf_token })
    }

    async fn _common_get<T: DeserializeOwned>(&self, url: impl reqwest::IntoUrl) -> Result<T> {
        self.client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json::<WrappedResponse<T>>()
            .await?
            .into()
    }

    /// Get the User ID of the logged in user.
    pub async fn self_user_id(&self) -> Result<Option<i32>> {
        let resp = self
            .client
            .get(BASE_URL_HTTPS)
            .send()
            .await?
            .error_for_status()?;
        let headers = resp.headers();
        Ok(headers
            .get("x-userid")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| <i32 as FromStr>::from_str(value).ok()))
    }

    /// Get the info of an user.
    pub async fn user_info(&self, user_id: i32) -> Result<UserInfo> {
        self._common_get(format!("{BASE_URL_HTTPS}/ajax/user/{user_id}?full=1")).await
    }

    /// Get the top works of an user.
    pub async fn user_top_works(&self, user_id: i32) -> Result<UserTopWorks> {
        self._common_get(format!("{BASE_URL_HTTPS}/ajax/user/{user_id}/profile/top")).await
    }

    /// Get all the works of an user.
    pub async fn user_all_works(&self, user_id: i32) -> Result<UserAllWorks> {
        self._common_get(format!("{BASE_URL_HTTPS}/ajax/user/{user_id}/profile/all")).await
    }

    /// Get the info of an illust.
    pub async fn illust_info(&self, illust_id: i32) -> Result<IllustInfo> {
        self._common_get(format!("{BASE_URL_HTTPS}/ajax/illust/{illust_id}")).await
    }

    /// Get the info of an illust.
    pub async fn illust_pages(&self, illust_id: i32) -> Result<Vec<IllustImage>> {
        self._common_get(format!("{BASE_URL_HTTPS}/ajax/illust/{illust_id}/pages")).await
    }

    async fn csrf_token(client: &Client) -> Result<String> {
        let resp = client
            .get(BASE_URL_HTTPS)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        let re = Regex::new(r#"token":"([^"])"#).unwrap();
        let caps = re
            .captures(&resp)
            .ok_or(crate::Error::Other("No CSRF Token Found"))?;
        let token = &caps[1];
        Ok(token.to_string())
    }
}
