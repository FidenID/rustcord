//! Klien REST untuk API Discord (endpoint sisi user).

use crate::models::{Channel, Guild, Message, User};
use anyhow::{Context, Result};

pub const BASE: &str = "https://discord.com/api/v10";

/// Discord menolak User-Agent yang bukan browser, jadi kita tiru UA browser umum.
pub const USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36 Rustcord/0.1";

#[derive(Clone)]
pub struct Api {
    http: reqwest::Client,
}

impl Api {
    pub fn new(token: &str) -> Result<Self> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            reqwest::header::HeaderValue::from_str(token)
                .context("token mengandung karakter tidak valid")?,
        );
        let http = reqwest::Client::builder()
            .default_headers(headers)
            .user_agent(USER_AGENT)
            .build()?;
        Ok(Self { http })
    }

    fn url(&self, path: &str) -> String {
        format!("{BASE}{path}")
    }

    pub async fn me(&self) -> Result<User> {
        let resp = self.http.get(self.url("/users/@me")).send().await?;
        let resp = check(resp).await?;
        Ok(resp.json().await?)
    }

    pub async fn guilds(&self) -> Result<Vec<Guild>> {
        let resp = self.http.get(self.url("/users/@me/guilds")).send().await?;
        let resp = check(resp).await?;
        Ok(resp.json().await?)
    }

    pub async fn guild_channels(&self, guild_id: &str) -> Result<Vec<Channel>> {
        let resp = self
            .http
            .get(self.url(&format!("/guilds/{guild_id}/channels")))
            .send()
            .await?;
        let resp = check(resp).await?;
        Ok(resp.json().await?)
    }

    pub async fn dm_channels(&self) -> Result<Vec<Channel>> {
        let resp = self.http.get(self.url("/users/@me/channels")).send().await?;
        let resp = check(resp).await?;
        Ok(resp.json().await?)
    }

    /// Mengembalikan pesan terbaru dulu (urutan API Discord); pemanggil yang membalik.
    pub async fn messages(
        &self,
        channel_id: &str,
        limit: u32,
        before: Option<&str>,
    ) -> Result<Vec<Message>> {
        let mut url = format!("/channels/{channel_id}/messages?limit={limit}");
        if let Some(b) = before {
            url.push_str(&format!("&before={b}"));
        }
        let resp = self
            .http
            .get(self.url(&url))
            .send()
            .await?;
        let resp = check(resp).await?;
        Ok(resp.json().await?)
    }

    pub async fn send_message(&self, channel_id: &str, content: &str) -> Result<Message> {
        let resp = self
            .http
            .post(self.url(&format!("/channels/{channel_id}/messages")))
            .json(&serde_json::json!({ "content": content }))
            .send()
            .await?;
        let resp = check(resp).await?;
        Ok(resp.json().await?)
    }

    pub async fn add_reaction(&self, channel_id: &str, message_id: &str, emoji: &str) -> Result<()> {
        let path = format!(
            "/channels/{channel_id}/messages/{message_id}/reactions/{}/@@me",
            urlencoding_encode(emoji)
        );
        let resp = self.http.put(self.url(&path)).send().await?;
        check(resp).await?;
        Ok(())
    }

    /// Unduh file dari CDN (ikon server, avatar, attachment).
    pub async fn download(&self, url: &str) -> Result<Vec<u8>> {
        let resp = self.http.get(url).send().await?;
        let resp = check(resp).await?;
        Ok(resp.bytes().await?.to_vec())
    }

    /// Daftar teman (endpoint khusus akun user).
    pub async fn relationships(&self) -> Result<Vec<crate::models::Relationship>> {
        let resp = self
            .http
            .get(self.url("/users/@me/relationships"))
            .send()
            .await?;
        let resp = check(resp).await?;
        Ok(resp.json().await?)
    }

    /// Daftar anggota guild (dibatasi limit).
    pub async fn guild_members(
        &self,
        guild_id: &str,
        limit: u32,
    ) -> Result<Vec<crate::models::GuildMember>> {
        let resp = self
            .http
            .get(self.url(&format!("/guilds/{guild_id}/members")))
            .query(&[("limit", limit.to_string())])
            .send()
            .await?;
        let resp = check(resp).await?;
        Ok(resp.json().await?)
    }
}

async fn check(resp: reqwest::Response) -> Result<reqwest::Response> {
    let status = resp.status();
    if status.is_success() {
        return Ok(resp);
    }
    let body = resp.text().await.unwrap_or_default();
    anyhow::bail!("HTTP {status}: {}", truncate(&body, 300))
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}

/// Encoding URL sederhana untuk emoji reaksi (unicode perlu di-encode).
fn urlencoding_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
