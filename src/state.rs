//! Pesan-pesan yang dikirim task background ke thread UI.

use crate::models::{Channel, Guild, Message, User};
use std::collections::HashMap;

/// Semua update state yang datang dari task async.
#[derive(Debug)]
pub enum UiMsg {
    Me(User),
    Guilds(Vec<Guild>),
    DmChannels(Vec<Channel>),
    GuildChannels { guild_id: String, channels: Vec<Channel> },
    Messages { channel_id: String, messages: Vec<Message> },
    Sent(Message),
    MessageCreate(Message),
    Status(String),
    LoginFailed(String),
    Disconnected(String),
    Image { url: String, bytes: Vec<u8> },
    Friends(Vec<User>),
    Presences(HashMap<String, String>),
    /// Posisi voice awal semua orang saat READY: guild -> user -> channel.
    VoiceSeed(HashMap<String, HashMap<String, Option<String>>>),
    PresenceUpdate { user_id: String, status: String },
    /// Preview pesan terakhir satu DM (nama pengirim, isi).
    DmPreview { channel_id: String, author: String, content: String },
    /// Daftar anggota sebuah guild.
    GuildMembers { guild_id: String, members: Vec<crate::models::GuildMember> },
    /// Gagal memuat pesan sebuah channel.
    LoadError { channel_id: String, error: String },
    /// Seseorang sedang mengetik di channel.
    Typing { channel_id: String, user_id: String },
    /// Halaman pesan lama (sebelum pesan pertama yang dimuat).
    OlderMessages { channel_id: String, messages: Vec<Message> },
    /// Update posisi voice siapa pun di sebuah guild.
    VoiceStateUpdate { guild_id: String, channel_id: Option<String>, user_id: String },
    /// Sesi suara berhasil tersambung (kita bisa dengar).
    VoiceConnected { guild_id: String, channel_id: String },
    /// Sesi suara gagal/berakhir.
    VoiceFailed(String),
    /// Sesi suara ditutup (normal).
    VoiceLeft,
}
