//! Struktur data domain Discord (subset yang dipakai Rustcord).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub global_name: Option<String>,
    #[serde(default)]
    pub discriminator: String,
    #[serde(default)]
    pub avatar: Option<String>,
    #[serde(default)]
    pub bot: bool,
}

impl User {
    /// Nama tampilan: global_name jika ada, lalu username.
    pub fn display_name(&self) -> &str {
        self.global_name.as_deref().unwrap_or(&self.username)
    }

    pub fn avatar_url(&self, size: u32) -> Option<String> {
        let hash = self.avatar.as_ref()?;
        let ext = if hash.starts_with("a_") { "gif" } else { "png" };
        Some(format!(
            "https://cdn.discordapp.com/avatars/{}/{}.{}?size={}",
            self.id, hash, ext, size
        ))
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Relationship {
    pub id: String,
    #[serde(default, rename = "type")]
    pub kind: u8,
    #[serde(default)]
    pub user: Option<User>,
    #[serde(default)]
    pub nickname: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub global_name: Option<String>,
    #[serde(default)]
    pub discriminator: Option<String>,
    #[serde(default)]
    pub avatar: Option<String>,
}

impl Relationship {
    pub fn to_user(&self) -> User {
        if let Some(u) = &self.user {
            let mut u = u.clone();
            if let Some(nick) = &self.nickname {
                u.global_name = Some(nick.clone());
            }
            return u;
        }
        User {
            id: self.id.clone(),
            username: self.username.clone().unwrap_or_default(),
            global_name: self.nickname.clone().or(self.global_name.clone()),
            discriminator: self.discriminator.clone().unwrap_or_default(),
            avatar: self.avatar.clone(),
            bot: false,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Guild {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub icon: Option<String>,
}

impl Guild {
    pub fn icon_url(&self, size: u32) -> Option<String> {
        let hash = self.icon.as_ref()?;
        let ext = if hash.starts_with("a_") { "gif" } else { "png" };
        Some(format!(
            "https://cdn.discordapp.com/icons/{}/{}.{}?size={}",
            self.id, hash, ext, size
        ))
    }
}

/// Tipe channel yang relevan untuk Rustcord.
pub const CHANNEL_TEXT: u8 = 0;
pub const CHANNEL_DM: u8 = 1;
pub const CHANNEL_VOICE: u8 = 2;
pub const CHANNEL_GROUP_DM: u8 = 3;
pub const CHANNEL_CATEGORY: u8 = 4;
pub const CHANNEL_ANNOUNCEMENT: u8 = 5;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    #[serde(rename = "type", default)]
    pub kind: u8,
    #[serde(default)]
    pub guild_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub topic: Option<String>,
    #[serde(default)]
    pub position: i32,
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub last_message_id: Option<String>,
    #[serde(default)]
    pub recipients: Vec<User>,
}

impl Channel {
    pub fn is_textlike(&self) -> bool {
        matches!(self.kind, CHANNEL_TEXT | CHANNEL_ANNOUNCEMENT | CHANNEL_DM | CHANNEL_GROUP_DM)
    }

    pub fn is_category(&self) -> bool {
        self.kind == CHANNEL_CATEGORY
    }

    /// Nama tampilan channel (DM memakai nama lawan bicara).
    pub fn display_name(&self) -> String {
        if let Some(name) = &self.name {
            if !name.is_empty() {
                return name.clone();
            }
        }
        match self.recipients.first() {
            Some(u) => u.display_name().to_string(),
            None => "unknown-channel".into(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GuildMember {
    #[serde(default)]
    pub user: Option<User>,
    #[serde(default)]
    pub nick: Option<String>,
}

impl GuildMember {
    pub fn to_user(&self) -> Option<User> {
        let mut u = self.user.clone()?;
        if let Some(nick) = &self.nick {
            u.global_name = Some(nick.clone());
        }
        Some(u)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Attachment {
    pub id: String,
    #[serde(default)]
    pub filename: String,
    #[serde(default)]
    pub content_type: Option<String>,
    pub url: String,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
}

impl Attachment {
    pub fn is_image(&self) -> bool {
        if let Some(ct) = &self.content_type {
            if ct.starts_with("image/") {
                return true;
            }
        }
        matches!(
            self.filename.rsplit('.').next().map(|e| e.to_ascii_lowercase()).as_deref(),
            Some("png") | Some("jpg") | Some("jpeg") | Some("gif") | Some("webp")
        )
    }
}

/// Satu reaksi pada pesan (emoji + jumlah).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Reaction {
    #[serde(default)]
    pub count: u32,
    #[serde(default)]
    pub me: bool,
    #[serde(default)]
    pub emoji: EmojiRef,
}

/// Referensi emoji (unicode atau kustom dari guild).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EmojiRef {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub animated: bool,
}

impl EmojiRef {
    /// Teks yang ditampilkan: unicode apa adanya, kustom jadi `:nama:`.
    pub fn label(&self) -> String {
        match &self.name {
            Some(n) if self.id.is_none() => n.clone(),
            Some(n) => format!(":{n}:"),
            None => "❔".into(),
        }
    }

    /// URL gambar emoji kustom dari CDN (None jika emoji unicode).
    pub fn image_url(&self) -> Option<String> {
        let id = self.id.as_ref()?;
        let ext = if self.animated { "gif" } else { "png" };
        Some(format!("https://cdn.discordapp.com/emojis/{id}.{ext}?size=32"))
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    #[serde(default)]
    pub channel_id: String,
    #[serde(default)]
    pub author: User,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub timestamp: String,
    #[serde(default)]
    pub edited_timestamp: Option<String>,
    #[serde(default)]
    pub mentions: Vec<User>,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    #[serde(default)]
    pub reactions: Vec<Reaction>,
    /// Pesan yang dibalas (untuk tipe pesan reply).
    #[serde(default)]
    pub referenced_message: Option<Box<Message>>,
}

impl Message {
    /// Snowflake id -> urutan waktu (semakin besar semakin baru).
    pub fn snowflake(&self) -> u128 {
        self.id.parse::<u128>().unwrap_or(0)
    }
}
