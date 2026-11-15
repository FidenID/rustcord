//! State aplikasi + logika (jembatan antara UI, REST API, dan gateway).

use crate::api::Api;
use crate::gateway;
use crate::models::*;
use crate::state::UiMsg;
use crate::ui;
use eframe::egui;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc;

/// Halaman yang sedang dilihat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Dms,
    Friends,
    Guild,
    /// Tampilan utama sedang menampilkan halaman voice channel.
    Voice,
}

pub struct App {
    pub token_input: String,
    pub status: String,
    pub error: Option<String>,
    pub connecting: bool,

    pub me: Option<User>,
    pub api: Option<Api>,

    pub guilds: Vec<Guild>,
    /// Channel milik guild terpilih; saat mode DM berisi daftar DM.
    pub channels: Vec<Channel>,
    pub dm_channels: Vec<Channel>,
    pub messages: Vec<Message>,

    pub selected_guild: Option<String>,
    pub selected_channel: Option<String>,
    pub view: View,

    pub composer: String,
    pub collapsed: HashSet<String>,
    pub unread: HashSet<String>,
    pub scroll_to_bottom: bool,

    /// Teman + status kehadiran (online/idle/dnd/offline).
    pub friends: Vec<User>,
    pub presences: HashMap<String, String>,
    pub filter_dms: String,
    pub filter_friends: String,
    pub friends_show_all: bool,

    /// Preview pesan terakhir per channel DM: (nama pengirim, isi).
    pub dm_previews: HashMap<String, (String, String)>,

    /// Cache anggota guild (id guild -> daftar user).
    pub guild_members: HashMap<String, Vec<User>>,
    fetched_members: HashSet<String>,

    /// Error pemuatan pesan per channel (mis. channel tidak bisa diakses).
    pub load_errors: HashMap<String, String>,

    /// "Sedang mengetik" per channel: (nama, sejak).
    pub typing: HashMap<String, (String, std::time::Instant)>,
    /// Status pagination pesan channel aktif.
    pub older_loading: bool,
    pub has_more: bool,

    /// Kirim perintah ke main gateway (join/leave voice).
    voice_cmd: Option<tokio::sync::mpsc::UnboundedSender<gateway::GatewayCmd>>,
    voice_cmd_rx: Option<tokio::sync::mpsc::UnboundedReceiver<gateway::GatewayCmd>>,
    /// Panel suara di sidebar: guild, channel, nama, sudah tersambung?
    pub voice_panel: Option<(String, String, String, bool)>,
    /// Waktu mulai tersambung (untuk timer di sidebar).
    pub voice_start: Option<std::time::Instant>,
    /// Deafen diri sendiri (tolak semua audio voice).
    pub self_deaf: bool,
    /// Posisi voice semua orang: guild -> user -> channel (None = keluar).
    pub voice_map: HashMap<String, HashMap<String, Option<String>>>,

    /// Cache gambar (ikon server, avatar, attachment) dari CDN Discord.
    pub textures: HashMap<String, egui::TextureHandle>,
    pending_images: HashSet<String>,
    failed_images: HashSet<String>,

    rt: tokio::runtime::Runtime,
    tx: mpsc::Sender<UiMsg>,
    rx: mpsc::Receiver<UiMsg>,
    egui_ctx: egui::Context,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let egui_ctx = cc.egui_ctx.clone();

        // Font: pakai Noto Sans dari sistem (visual lebih mirip Discord).
        ui::load_fonts(&egui_ctx);

        // Tema gelap ala Discord.
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = ui::CHAT_BG;
        visuals.window_fill = ui::SIDEBAR_BG;
        visuals.extreme_bg_color = ui::INPUT_BG; // latar composer
        visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(6);
        visuals.widgets.active.corner_radius = egui::CornerRadius::same(6);
        visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(6);
        egui_ctx.set_visuals(visuals);

        let (tx, rx) = mpsc::channel();
        let (vcmd_tx, vcmd_rx) = tokio::sync::mpsc::unbounded_channel();
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("gagal membuat tokio runtime");

        let mut app = Self {
            token_input: String::new(),
            status: String::new(),
            error: None,
            connecting: false,
            me: None,
            api: None,
            guilds: Vec::new(),
            channels: Vec::new(),
            dm_channels: Vec::new(),
            messages: Vec::new(),
            selected_guild: None,
            selected_channel: None,
            view: View::Dms,
            composer: String::new(),
            collapsed: HashSet::new(),
            unread: HashSet::new(),
            scroll_to_bottom: false,
            friends: Vec::new(),
            presences: HashMap::new(),
            filter_dms: String::new(),
            filter_friends: String::new(),
            friends_show_all: false,
            dm_previews: HashMap::new(),
            guild_members: HashMap::new(),
            fetched_members: HashSet::new(),
            load_errors: HashMap::new(),
            typing: HashMap::new(),
            older_loading: false,
            has_more: true,
            voice_cmd: Some(vcmd_tx.clone()),
            voice_cmd_rx: Some(vcmd_rx),
            voice_panel: None,
            voice_start: None,
            self_deaf: false,
            voice_map: HashMap::new(),
            textures: HashMap::new(),
            pending_images: HashSet::new(),
            failed_images: HashSet::new(),
            rt,
            tx: tx.clone(),
            rx,
            egui_ctx,
        };

        // Auto-login jika token tersimpan.
        if let Some(token) = load_token() {
            app.token_input = token;
            app.login();
        }
        app
    }

    // ------------------------------------------------------------- login

    pub fn login(&mut self) {
        let token = self.token_input.trim().to_string();
        if token.is_empty() {
            self.error = Some("Token masih kosong.".into());
            return;
        }
        let api = match Api::new(&token) {
            Ok(api) => api,
            Err(e) => {
                self.error = Some(format!("Token tidak valid: {e}"));
                return;
            }
        };
        self.api = Some(api.clone());
        self.error = None;
        self.connecting = true;
        self.status = "Menghubungkan ke Discord...".into();

        let tx = self.tx.clone();
        let ctx = self.egui_ctx.clone();
        let vcmd_rx = self.voice_cmd_rx.take();
        self.rt.spawn(async move {
            match api.me().await {
                Ok(me) => {
                    eprintln!("[api] /users/@me OK: {}", me.username);
                    let _ = tx.send(UiMsg::Me(me));
                    ctx.request_repaint();

                    match api.guilds().await {
                        Ok(g) => {
                            eprintln!("[api] guilds OK: {} guild", g.len());
                            let _ = tx.send(UiMsg::Guilds(g));
                        }
                        Err(e) => eprintln!("[api] guilds GAGAL: {e}"),
                    }
                    ctx.request_repaint();
                    match api.dm_channels().await {
                        Ok(d) => {
                            eprintln!("[api] dm OK: {} dm", d.len());
                            let _ = tx.send(UiMsg::DmChannels(d));
                        }
                        Err(e) => eprintln!("[api] dm GAGAL: {e}"),
                    }
                    ctx.request_repaint();
                    match api.relationships().await {
                        Ok(r) => {
                            let users: Vec<crate::models::User> =
                                r.iter().map(|x| x.to_user()).collect();
                            eprintln!("[api] teman OK: {} teman", users.len());
                            let _ = tx.send(UiMsg::Friends(users));
                        }
                        Err(e) => eprintln!("[api] teman GAGAL: {e}"),
                    }
                    ctx.request_repaint();

                    // Gateway berjalan selamanya (auto-reconnect di dalamnya).
                    gateway::run(token, tx.clone(), ctx.clone(), vcmd_rx.unwrap()).await;
                }
                Err(e) => {
                    let _ = tx.send(UiMsg::LoginFailed(format!("Login gagal: {e}")));
                    ctx.request_repaint();
                }
            }
        });
    }

    // ---------------------------------------------------------- navigasi

    /// Apakah user diketahui sedang daring (dari data presence).
    pub fn is_online(&self, user_id: &str) -> bool {
        matches!(
            self.presences.get(user_id).map(|s| s.as_str()),
            Some("online") | Some("idle") | Some("dnd")
        )
    }

    pub fn select_dms(&mut self) {
        self.view = View::Dms;
        self.selected_guild = None;
        self.selected_channel = None;
        self.messages.clear();
        let mut dms = self.dm_channels.clone();
        dms.sort_by_key(|c| std::cmp::Reverse(last_msg_snowflake(c)));
        let first = dms.first().map(|c| c.id.clone());
        self.channels = dms;
        if let Some(first) = first {
            self.select_channel(first);
        }
    }

    pub fn select_guild(&mut self, guild_id: String) {
        if self.selected_guild.as_deref() == Some(&guild_id) {
            return;
        }
        self.selected_guild = Some(guild_id.clone());
        self.view = View::Guild;
        self.selected_channel = None;
        self.messages.clear();
        self.channels.clear();

        let Some(api) = self.api.clone() else { return };
        let tx = self.tx.clone();
        let ctx = self.egui_ctx.clone();
        self.rt.spawn(async move {
            match api.guild_channels(&guild_id).await {
                Ok(mut chs) => {
                    eprintln!("[api] channels OK: {} channel di guild {guild_id}", chs.len());
                    chs.sort_by_key(|c| (c.position, c.id.clone()));
                    let _ = tx.send(UiMsg::GuildChannels { guild_id, channels: chs });
                }
                Err(e) => eprintln!("[api] channels GAGAL guild {guild_id}: {e}"),
            }
            ctx.request_repaint();
        });
    }

    pub fn select_channel(&mut self, channel_id: String) {
        if self.selected_channel.as_deref() == Some(&channel_id) {
            return;
        }
        self.selected_channel = Some(channel_id.clone());
        self.messages.clear();
        self.unread.remove(&channel_id);
        self.load_errors.remove(&channel_id);
        self.older_loading = false;
        self.has_more = true;
        self.scroll_to_bottom();

        // Ambil daftar anggota guild sekali per guild (untuk panel kanan).
        if let Some(gid) = self.selected_guild.clone() {
            if !self.fetched_members.contains(&gid) {
                self.fetched_members.insert(gid.clone());
                if let Some(api) = self.api.clone() {
                    let tx = self.tx.clone();
                    let ctx = self.egui_ctx.clone();
                    self.rt.spawn(async move {
                        match api.guild_members(&gid, 200).await {
                            Ok(ms) => {
                                eprintln!("[api] anggota OK: {} di guild {gid}", ms.len());
                                let _ = tx.send(UiMsg::GuildMembers { guild_id: gid, members: ms });
                            }
                            Err(e) => eprintln!("[api] anggota GAGAL guild {gid}: {e}"),
                        }
                        ctx.request_repaint();
                    });
                }
            }
        }

        let Some(api) = self.api.clone() else { return };
        let tx = self.tx.clone();
        let ctx = self.egui_ctx.clone();
        self.rt.spawn(async move {
            match api.messages(&channel_id, 50, None).await {
                Ok(mut msgs) => {
                    eprintln!("[api] pesan OK: {} pesan di {channel_id}", msgs.len());
                    msgs.reverse(); // API memberi terbaru dulu -> balik jadi lama ke baru
                    let _ = tx.send(UiMsg::Messages { channel_id, messages: msgs });
                }
                Err(e) => {
                    eprintln!("[api] pesan GAGAL di {channel_id}: {e}");
                    let _ = tx.send(UiMsg::LoadError {
                        channel_id,
                        error: format!("{e}"),
                    });
                }
            }
            ctx.request_repaint();
        });
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_to_bottom = true;
    }

    /// Gabung / keluar voice channel (klik ulang channel yang sama = keluar).
    pub fn join_voice(&mut self, channel_id: String) {
        let Some(cmd) = &self.voice_cmd else { return };
        let Some(ch) = self.channels.iter().find(|c| c.id == channel_id) else { return };
        let Some(guild_id) = self.selected_guild.clone() else { return };
        let name = ch.display_name();

        // Klik channel yang sama = putus.
        if let Some((_, cur_ch, _, _)) = &self.voice_panel {
            if cur_ch == &channel_id {
                self.leave_voice();
                return;
            }
        }
        let _ = cmd.send(gateway::GatewayCmd::VoiceJoin {
            guild_id,
            channel_id: channel_id.clone(),
        });
        self.voice_panel = Some((guild_id_of_current(self), channel_id, name, false));
        self.voice_start = None;
        self.self_deaf = false;
        self.view = View::Voice; // tampilkan halaman voice di area utama

        fn guild_id_of_current(app: &App) -> String {
            app.selected_guild.clone().unwrap_or_default()
        }
    }

    pub fn leave_voice(&mut self) {
        if let Some(cmd) = &self.voice_cmd {
            let _ = cmd.send(gateway::GatewayCmd::VoiceLeave);
        }
        self.voice_panel = None;
        self.voice_start = None;
        if self.view == View::Voice {
            self.view = View::Dms;
        }
    }

    /// Toggle deafen (tolak semua audio voice).
    pub fn toggle_deaf(&mut self) {
        self.self_deaf = !self.self_deaf;
        if let Some(cmd) = &self.voice_cmd {
            let _ = cmd.send(gateway::GatewayCmd::VoiceSetFlags {
                mute: true, // Phase V1: mic selalu mati
                deaf: self.self_deaf,
            });
        }
    }

    /// Muat halaman pesan sebelum halaman pertama channel aktif.
    pub fn load_older(&mut self) {
        if self.older_loading || !self.has_more {
            return;
        }
        let Some(channel_id) = self.selected_channel.clone() else { return };
        let Some(oldest) = self.messages.first().map(|m| m.id.clone()) else { return };
        let Some(api) = self.api.clone() else { return };
        self.older_loading = true;

        let tx = self.tx.clone();
        let ctx = self.egui_ctx.clone();
        self.rt.spawn(async move {
            match api.messages(&channel_id, 50, Some(&oldest)).await {
                Ok(mut msgs) => {
                    msgs.reverse(); // API terbaru dulu -> urutkan lama ke baru
                    eprintln!("[api] pesan lama OK: {} di {channel_id}", msgs.len());
                    let _ = tx.send(UiMsg::OlderMessages { channel_id, messages: msgs });
                }
                Err(e) => eprintln!("[api] pesan lama GAGAL di {channel_id}: {e}"),
            }
            ctx.request_repaint();
        });
    }

    pub fn set_friends_view(&mut self) {
        self.view = View::Friends;
    }

    pub fn toggle_category(&mut self, id: String) {
        if !self.collapsed.remove(&id) {
            self.collapsed.insert(id);
        }
    }

    // -------------------------------------------------------- aksi chat

    /// Ambil pesan terakhir tiap DM (dibatasi 50 pertama, berurutan + jeda
    /// agar tidak kena rate limit) untuk subtitle daftar DM.
    fn fetch_dm_previews(&mut self) {
        let Some(api) = self.api.clone() else { return };
        let targets: Vec<String> = self
            .dm_channels
            .iter()
            .filter(|c| c.last_message_id.is_some())
            .take(50)
            .map(|c| c.id.clone())
            .collect();
        if targets.is_empty() {
            return;
        }
        let tx = self.tx.clone();
        let ctx = self.egui_ctx.clone();
        self.rt.spawn(async move {
            for cid in targets {
                match api.messages(&cid, 1, None).await {
                    Ok(msgs) => {
                        if let Some(m) = msgs.first() {
                            let _ = tx.send(UiMsg::DmPreview {
                                channel_id: cid.clone(),
                                author: m.author.display_name().to_string(),
                                content: m.content.clone(),
                            });
                            ctx.request_repaint();
                        }
                    }
                    Err(e) => {
                        eprintln!("[api] preview DM {cid} GAGAL: {e}");
                        break; // kemungkinan rate limit; hentikan serangkaian ini
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
        });
    }

    /// Perbarui preview DM saat pesan baru masuk/keluar.
    fn update_preview_from_message(&mut self, m: &Message) {
        if m.content.is_empty() {
            return;
        }
        let author = if Some(&m.author.id) == self.me.as_ref().map(|u| &u.id) {
            "Kamu".to_string()
        } else {
            m.author.display_name().to_string()
        };
        self.dm_previews
            .insert(m.channel_id.clone(), (author, m.content.clone()));
    }

    pub fn send(&mut self) {
        let content = self.composer.trim().to_string();
        if content.is_empty() {
            return;
        }
        let Some(channel_id) = self.selected_channel.clone() else { return };
        let Some(api) = self.api.clone() else { return };
        self.composer.clear();

        let tx = self.tx.clone();
        let ctx = self.egui_ctx.clone();
        self.rt.spawn(async move {
            match api.send_message(&channel_id, &content).await {
                Ok(m) => {
                    let _ = tx.send(UiMsg::Sent(m));
                }
                Err(e) => {
                    let _ = tx.send(UiMsg::Status(format!("Gagal mengirim: {e}")));
                }
            }
            ctx.request_repaint();
        });
    }

    // ------------------------------------------------------------ gambar

    /// Pastikan gambar tersedia di cache; mulai unduh jika belum.
    pub fn ensure_image(&mut self, url: String) {
        if self.textures.contains_key(&url)
            || self.pending_images.contains(&url)
            || self.failed_images.contains(&url)
        {
            return;
        }
        self.pending_images.insert(url.clone());
        let Some(api) = self.api.clone() else {
            self.pending_images.remove(&url);
            return;
        };
        let tx = self.tx.clone();
        let ctx = self.egui_ctx.clone();
        self.rt.spawn(async move {
            match api.download(&url).await {
                Ok(bytes) => {
                    eprintln!("[img] OK   {url} ({} bytes)", bytes.len());
                    let _ = tx.send(UiMsg::Image { url, bytes });
                }
                Err(e) => {
                    eprintln!("[img] GAGAL {url}: {e}");
                    let _ = tx.send(UiMsg::Image { url, bytes: Vec::new() });
                }
            }
            ctx.request_repaint();
        });
    }

    /// Kumpulkan URL gambar yang sedang terlihat agar mulai diunduh.
    fn prewarm_images(&mut self) {
        let mut urls: Vec<String> = Vec::new();
        if let Some(me) = &self.me {
            if let Some(u) = me.avatar_url(64) {
                urls.push(u);
            }
        }
        for g in &self.guilds {
            if let Some(u) = g.icon_url(64) {
                urls.push(u);
            }
        }
        for c in &self.channels {
            if let Some(u) = c.recipients.first().and_then(|u| u.avatar_url(64)) {
                urls.push(u);
            }
        }
        for f in &self.friends {
            if let Some(u) = f.avatar_url(64) {
                urls.push(u);
            }
        }
        for m in &self.messages {
            if let Some(u) = m.author.avatar_url(64) {
                urls.push(u);
            }
            for a in &m.attachments {
                if a.is_image() {
                    urls.push(a.url.clone());
                }
            }
            for r in &m.reactions {
                if let Some(u) = r.emoji.image_url() {
                    urls.push(u);
                }
            }
        }
        for u in urls {
            self.ensure_image(u);
        }
    }

    fn decode_image(&mut self, url: &str, bytes: &[u8]) {
        match image::load_from_memory(bytes) {
            Ok(img) => {
                eprintln!("[img] decode OK {url} ({}x{})", img.width(), img.height());
                // Batasi ukuran agar hemat memori GPU.
                let img = if img.width() > 512 || img.height() > 512 {
                    img.resize(512, 512, image::imageops::FilterType::Triangle)
                } else {
                    img
                };
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let color = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
                let handle =
                    self.egui_ctx.load_texture(url.to_string(), color, egui::TextureOptions::LINEAR);
                self.textures.insert(url.to_string(), handle);
            }
            Err(_) => {
                eprintln!("[img] decode GAGAL {url}");
                self.failed_images.insert(url.to_string());
            }
        }
        self.pending_images.remove(url);
    }

    // ----------------------------------------------------------- helpers

    pub fn push_message(&mut self, m: Message) {
        if self.messages.iter().any(|x| x.id == m.id) {
            return;
        }
        self.messages.push(m);
        self.messages.sort_by_key(|m| m.snowflake());
    }

    pub fn channel_by_id(&self, id: &str) -> Option<&Channel> {
        self.channels.iter().find(|c| c.id == id)
    }

    /// Cari nama tampilan user dari cache yang ada (teman, DM, pesan, anggota).
    pub fn resolve_name(&self, user_id: &str) -> String {
        if self.me.as_ref().map(|u| u.id.as_str()) == Some(user_id) {
            return "Kamu".into();
        }
        if let Some(f) = self.friends.iter().find(|u| u.id == user_id) {
            return f.display_name().to_string();
        }
        for c in self.channels.iter().chain(self.dm_channels.iter()) {
            if let Some(u) = c.recipients.iter().find(|u| u.id == user_id) {
                return u.display_name().to_string();
            }
        }
        if let Some(m) = self.messages.iter().find(|m| m.author.id == user_id) {
            return m.author.display_name().to_string();
        }
        for ms in self.guild_members.values() {
            if let Some(u) = ms.iter().find(|u| u.id == user_id) {
                return u.display_name().to_string();
            }
        }
        "Seseorang".into()
    }

    fn apply(&mut self, msg: UiMsg) {
        match msg {
            UiMsg::Me(me) => {
                self.me = Some(me);
                self.status = "Memuat server...".into();
                save_token(self.token_input.trim());
            }
            UiMsg::Guilds(g) => {
                self.guilds = g;
                self.status = "Terhubung".into();
                self.connecting = false;
            }
            UiMsg::DmChannels(dms) => {
                self.dm_channels = dms;
                if self.selected_guild.is_none() && self.selected_channel.is_none() {
                    self.select_dms();
                }
                self.fetch_dm_previews();
            }
            UiMsg::GuildChannels { guild_id, channels } => {
                if self.selected_guild.as_deref() == Some(&guild_id) {
                    self.channels = channels;
                    if self.selected_channel.is_none() {
                        if let Some(first) =
                            self.channels.iter().find(|c| c.is_textlike()).map(|c| c.id.clone())
                        {
                            self.select_channel(first);
                        }
                    }
                }
            }
            UiMsg::Messages { channel_id, messages } => {
                if self.selected_channel.as_deref() == Some(&channel_id) {
                    self.messages = messages;
                    self.has_more = true;
                    self.older_loading = false;
                    self.scroll_to_bottom = true;
                }
            }
            UiMsg::Sent(m) => {
                self.update_preview_from_message(&m);
                self.push_message(m);
            }
            UiMsg::MessageCreate(m) => {
                let cid = m.channel_id.clone();
                if self.selected_channel.as_deref() == Some(&cid) {
                    self.update_preview_from_message(&m);
                    self.push_message(m);
                    self.scroll_to_bottom = true;
                } else {
                    self.unread.insert(cid);
                    self.update_preview_from_message(&m);
                }
            }
            UiMsg::Status(s) => {
                self.status = s;
            }
            UiMsg::LoginFailed(e) => {
                self.connecting = false;
                self.me = None;
                self.error = Some(e);
                self.status.clear();
            }
            UiMsg::Disconnected(e) => {
                self.status = format!("Terputus: {e} — mencoba ulang dalam 5 detik...");
            }
            UiMsg::Image { url, bytes } => {
                if bytes.is_empty() {
                    self.pending_images.remove(&url);
                    self.failed_images.insert(url);
                } else {
                    self.decode_image(&url, &bytes);
                }
            }
            UiMsg::Friends(users) => {
                self.friends = users;
            }
            UiMsg::Presences(m) => {
                for (k, v) in m {
                    self.presences.insert(k, v);
                }
            }
            UiMsg::PresenceUpdate { user_id, status } => {
                self.presences.insert(user_id, status);
            }
            UiMsg::DmPreview { channel_id, author, content } => {
                if !content.is_empty() {
                    self.dm_previews.insert(channel_id, (author, content));
                }
            }
            UiMsg::GuildMembers { guild_id, members } => {
                let mut users: Vec<User> =
                    members.iter().filter_map(|m| m.to_user()).collect();
                users.sort_by(|a, b| {
                    let oa = self.is_online(&a.id);
                    let ob = self.is_online(&b.id);
                    ob.cmp(&oa)
                        .then_with(|| a.display_name().to_lowercase().cmp(&b.display_name().to_lowercase()))
                });
                self.guild_members.insert(guild_id, users);
            }
            UiMsg::LoadError { channel_id, error } => {
                // Pesan error pendek saja untuk ditampilkan di UI.
                let pendek = if error.contains("403") || error.contains("Access") {
                    "Tidak punya akses ke channel ini.".to_string()
                } else {
                    error
                };
                self.load_errors.insert(channel_id, pendek);
            }
            UiMsg::OlderMessages { channel_id, messages } => {
                if self.selected_channel.as_deref() == Some(&channel_id) {
                    self.has_more = messages.len() >= 50;
                    self.older_loading = false;
                    let mut known: HashSet<String> =
                        self.messages.iter().map(|m| m.id.clone()).collect();
                    for m in messages {
                        if known.insert(m.id.clone()) {
                            self.messages.push(m);
                        }
                    }
                    self.messages.sort_by_key(|m| m.snowflake());
                }
            }
            UiMsg::Typing { channel_id, user_id } => {
                let name = self.resolve_name(&user_id);
                self.typing
                    .insert(channel_id, (name, std::time::Instant::now()));
            }
            UiMsg::VoiceSeed(seed) => {
                self.voice_map = seed;
            }
            UiMsg::VoiceStateUpdate { guild_id, channel_id, user_id } => {
                self.voice_map
                    .entry(guild_id)
                    .or_default()
                    .insert(user_id, channel_id);
            }
            UiMsg::VoiceConnected { guild_id, channel_id } => {
                if let Some((_, ch, _name, ok)) = &mut self.voice_panel {
                    if ch == &channel_id {
                        *ok = true;
                    }
                } else {
                    let name = self
                        .channels
                        .iter()
                        .find(|c| c.id == channel_id)
                        .map(|c| c.display_name())
                        .unwrap_or_else(|| "voice".into());
                    self.voice_panel = Some((guild_id, channel_id, name, true));
                }
                if self.voice_start.is_none() {
                    self.voice_start = Some(std::time::Instant::now());
                }
                self.status = "Terhubung ke voice 🔊".into();
            }
            UiMsg::VoiceFailed(e) => {
                self.voice_panel = None;
                self.voice_start = None;
                self.status = format!("Voice gagal: {e}");
                if self.view == View::Voice {
                    self.view = View::Dms;
                }
            }
            UiMsg::VoiceLeft => {
                self.voice_panel = None;
                self.voice_start = None;
            }
        }
    }

    fn drain_messages(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            self.apply(msg);
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_messages();
        // Buang indikator mengetik yang sudah kedaluwarsa (>8 dtk).
        self.typing
            .retain(|_, (_, t)| t.elapsed() < std::time::Duration::from_secs(8));
        if !self.typing.is_empty() {
            ctx.request_repaint_after(std::time::Duration::from_secs(1));
        }
        self.prewarm_images();
        ui::draw(self, ctx);
    }
}

// ------------------------------------------------------------ penyimpanan

fn config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".config/rustcord/token.txt")
}

pub fn load_token() -> Option<String> {
    let s = std::fs::read_to_string(config_path()).ok()?;
    let s = s.trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

pub fn save_token(token: &str) {
    let path = config_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, token);
}

fn last_msg_snowflake(c: &Channel) -> u128 {
    c.last_message_id.as_deref().and_then(|s| s.parse::<u128>().ok()).unwrap_or(0)
}
