//! UI ala Discord: titlebar -> rail server -> sidebar -> chat/teman.
//! v3: titlebar custom, halaman Teman, panel user, search DM.

use crate::app::{App, View};
use crate::models::{self, Attachment, Message, User};
use chrono::{DateTime, Datelike, Local};
use eframe::egui::{self, Align2, Color32, FontId, Vec2};
use std::collections::HashMap;

// Palet warna Discord dark theme
pub const TITLEBAR_BG: Color32 = Color32::from_rgb(0x11, 0x12, 0x14);
pub const RAIL_BG: Color32 = Color32::from_rgb(0x1e, 0x1f, 0x22);
pub const SIDEBAR_BG: Color32 = Color32::from_rgb(0x2b, 0x2d, 0x31);
pub const CHAT_BG: Color32 = Color32::from_rgb(0x31, 0x33, 0x38);
pub const INPUT_BG: Color32 = Color32::from_rgb(0x38, 0x3a, 0x40);
pub const USER_PANEL_BG: Color32 = Color32::from_rgb(0x23, 0x24, 0x28);
pub const CODE_BG: Color32 = Color32::from_rgb(0x22, 0x24, 0x2a);
pub const SELECTED_BG: Color32 = Color32::from_rgb(0x40, 0x42, 0x4a);
pub const ROW_HOVER: Color32 = Color32::from_rgb(0x35, 0x37, 0x3e);
pub const ACCENT: Color32 = Color32::from_rgb(0x58, 0x65, 0xf2);
pub const MENTION: Color32 = Color32::from_rgb(0x8b, 0x93, 0xff);
pub const LINK: Color32 = Color32::from_rgb(0x00, 0xa8, 0xfc);
pub const TEXT_MAIN: Color32 = Color32::from_rgb(0xdb, 0xdc, 0xde);
pub const TEXT_DIM: Color32 = Color32::from_rgb(0x94, 0x98, 0xa0);
pub const RED: Color32 = Color32::from_rgb(0xf2, 0x3f, 0x3f);
pub const GREEN: Color32 = Color32::from_rgb(0x23, 0xa5, 0x59);
pub const YELLOW: Color32 = Color32::from_rgb(0xf0, 0xb1, 0x32);
pub const GRAY: Color32 = Color32::from_rgb(0x80, 0x84, 0x8e);

const AVATAR_PALETTE: [Color32; 6] = [
    Color32::from_rgb(0x58, 0x65, 0xf2),
    Color32::from_rgb(0x23, 0xa5, 0x59),
    Color32::from_rgb(0xf0, 0xb1, 0x32),
    Color32::from_rgb(0xeb, 0x45, 0x9b),
    Color32::from_rgb(0x00, 0xa8, 0xfc),
    Color32::from_rgb(0xe6, 0x74, 0x25),
];

const UV_FULL: egui::Rect = egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::new(1.0, 1.0));

pub fn draw(app: &mut App, ctx: &egui::Context) {
    if app.me.is_none() {
        login_ui(app, ctx);
    } else {
        main_ui(app, ctx);
    }
}

// ---------------------------------------------------------------- font

/// Muat font Noto Sans dari sistem (pengganti Inter) untuk tampilan lebih rapi.
pub fn load_fonts(ctx: &egui::Context) {
    use std::sync::Arc;
    let mut fonts = egui::FontDefinitions::default();

    fn add(fonts: &mut egui::FontDefinitions, path: &str, name: &str) -> bool {
        match std::fs::read(path) {
            Ok(data) => {
                fonts
                    .font_data
                    .insert(name.to_string(), Arc::new(egui::FontData::from_owned(data)));
                true
            }
            Err(_) => false,
        }
    }

    let has_sans = add(&mut fonts, "/usr/share/fonts/noto/NotoSans-Regular.ttf", "noto_sans");
    add(&mut fonts, "/usr/share/fonts/noto/NotoSans-Bold.ttf", "noto_sans_bold");
    add(&mut fonts, "/usr/share/fonts/noto/NotoSans-Italic.ttf", "noto_sans_italic");
    let has_mono =
        add(&mut fonts, "/usr/share/fonts/noto/NotoSansMono-Regular.ttf", "noto_mono");

    if has_sans {
        if let Some(fam) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
            fam.insert(0, "noto_sans".into());
            fam.push("noto_sans_bold".into());
            fam.push("noto_sans_italic".into());
        }
    }
    if has_mono {
        if let Some(fam) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
            fam.insert(0, "noto_mono".into());
        }
    }
    ctx.set_fonts(fonts);
}

// --------------------------------------------------------------- login

fn login_ui(app: &mut App, ctx: &egui::Context) {
    egui::CentralPanel::default()
        .frame(egui::Frame::default().fill(RAIL_BG))
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(70.0);
                ui.label(egui::RichText::new("Rustcord").size(34.0).strong().color(TEXT_MAIN));
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("Klien Discord native — dibuat dengan Rust 🦀")
                        .size(13.0)
                        .color(TEXT_DIM),
                );
                ui.add_space(26.0);

                // Kartu login
                egui::Frame::default()
                    .fill(SIDEBAR_BG)
                    .corner_radius(egui::CornerRadius::same(12))
                    .inner_margin(egui::Margin::same(24))
                    .show(ui, |ui| {
                        ui.set_width(380.0);
                        ui.label(
                            egui::RichText::new("Token akun Discord")
                                .size(12.0)
                                .color(TEXT_DIM),
                        );
                        ui.add_space(4.0);
                        let token_edit = egui::TextEdit::singleline(&mut app.token_input)
                            .password(true);
                        let resp = ui.add(token_edit);
                        let enter =
                            resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

                        ui.add_space(12.0);
                        let login_btn = egui::Button::new(egui::RichText::new("Masuk").strong())
                            .fill(ACCENT);
                        let clicked = ui.add(login_btn).clicked();
                        ui.add_space(8.0);

                        if clicked || enter {
                            app.login();
                        }

                        if let Some(err) = &app.error {
                            ui.label(egui::RichText::new(err).size(12.0).color(RED));
                        }
                        if app.connecting {
                            ui.horizontal(|ui| {
                                ui.add(egui::Spinner::new().size(18.0));
                                ui.label(
                                    egui::RichText::new(&app.status).size(12.0).color(TEXT_DIM),
                                );
                            });
                        }
                    });

                ui.add_space(14.0);
                ui.label(
                    egui::RichText::new(
                        "Ambil token: Discord web → F12 → Network → Ctrl+R →\nklik request → header Authorization",
                    )
                    .size(11.0)
                    .color(TEXT_DIM),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(
                        "⚠ Klien unofficial berisiko banned akun (pemakaian pribadi).",
                    )
                    .size(11.0)
                    .color(RED),
                );
            });
        });
}

// ---------------------------------------------------------------- main

fn main_ui(app: &mut App, ctx: &egui::Context) {
    title_bar(app, ctx);
    guild_rail(app, ctx);
    channel_sidebar(app, ctx);
    member_panel(app, ctx);
    chat_panel(app, ctx);
}

// ------------------------------------------------------------ titlebar

fn window_title(app: &App) -> String {
    if app.view == View::Friends {
        return "Teman".into();
    }
    if let Some(cid) = &app.selected_channel {
        if let Some(ch) = app.channel_by_id(cid) {
            return ch.display_name();
        }
    }
    "Pesan Langsung".into()
}

fn title_bar(app: &mut App, ctx: &egui::Context) {
    egui::TopBottomPanel::top("titlebar")
        .exact_height(34.0)
        .resizable(false)
        .frame(egui::Frame::default().fill(TITLEBAR_BG))
        .show(ctx, |ui| {
            let size = Vec2::new(ui.available_width(), ui.available_height());
            let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());

            if resp.dragged() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            if resp.double_clicked() {
                let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
            }

            let painter = ui.painter();
            let y = rect.center().y;

            // Panah back/forward (dekoratif)
            let dim = Color32::from_rgb(0x4e, 0x50, 0x58);
            painter.text(
                egui::Pos2::new(rect.left() + 16.0, y),
                Align2::CENTER_CENTER,
                "‹",
                FontId::proportional(18.0),
                dim,
            );
            painter.text(
                egui::Pos2::new(rect.left() + 34.0, y),
                Align2::CENTER_CENTER,
                "›",
                FontId::proportional(18.0),
                dim,
            );

            // Judul di tengah
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                window_title(app),
                FontId::proportional(13.0),
                TEXT_MAIN,
            );

            // Tombol jendela: minimize, maximize, close
            let buttons = ["–", "□", "×"];
            for (i, icon) in buttons.iter().enumerate() {
                let w = 42.0;
                let is_close = i == 2;
                let brect = egui::Rect::from_min_size(
                    egui::Pos2::new(rect.right() - w * (i as f32 + 1.0), rect.top()),
                    Vec2::new(w, rect.height()),
                );
                let hover = ui.rect_contains_pointer(brect);
                if hover {
                    let bg = if is_close {
                        RED
                    } else {
                        Color32::from_rgba_unmultiplied(255, 255, 255, 22)
                    };
                    painter.rect_filled(brect, egui::CornerRadius::same(0), bg);
                }
                painter.text(
                    brect.center(),
                    Align2::CENTER_CENTER,
                    *icon,
                    FontId::proportional(if is_close { 14.0 } else { 11.0 }),
                    TEXT_MAIN,
                );
                let bresp = ui
                    .interact(brect, egui::Id::new(format!("titlebar_btn_{i}")), egui::Sense::click());
                if bresp.clicked() {
                    match i {
                        2 => ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close),
                        1 => {
                            let maximized =
                                ui.input(|i| i.viewport().maximized.unwrap_or(false));
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                        }
                        _ => ui.ctx().send_viewport_cmd(egui::ViewportCommand::Minimized(true)),
                    }
                }
            }
        });
}

// ---------------------------------------------------------- guild rail

fn guild_rail(app: &mut App, ctx: &egui::Context) {
    egui::SidePanel::left("rail")
        .exact_width(72.0)
        .resizable(false)
        .frame(
            egui::Frame::default()
                .fill(RAIL_BG)
                .inner_margin(egui::Margin::symmetric(0, 8)),
        )
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                // Tombol DM (beranda) — pakai avatar sendiri jika ada.
                let dm_selected = app.view != View::Guild;
                let me_avatar = app.me.as_ref().and_then(|m| m.avatar_url(64));
                let me_tex = me_avatar.as_ref().and_then(|u| app.textures.get(u));
                let resp = guild_button(ui, "@", me_tex, dm_selected, "home");
                if resp.clicked() {
                    app.select_dms();
                }

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(4.0);
                    let mut action: Option<String> = None;
                    for g in &app.guilds {
                        let tex =
                            g.icon_url(64).as_ref().and_then(|u| app.textures.get(u));
                        let selected = app.selected_guild.as_deref() == Some(g.id.as_str());
                        let resp = guild_button(ui, &g.name, tex, selected, &g.id);
                        if resp.clicked() {
                            action = Some(g.id.clone());
                        }
                    }
                    if let Some(id) = action {
                        app.select_guild(id);
                    }
                });
            });
        });
}

fn guild_button(
    ui: &mut egui::Ui,
    name: &str,
    tex: Option<&egui::TextureHandle>,
    selected: bool,
    seed: &str,
) -> egui::Response {
    let size = Vec2::splat(48.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let center = rect.center();
    let painter = ui.painter();

    if selected {
        // Pill putih di tepi kiri ala Discord
        painter.rect_filled(
            egui::Rect::from_min_size(
                egui::Pos2::new(rect.left() - 8.0, center.y - 11.0),
                Vec2::new(4.0, 22.0),
            ),
            egui::CornerRadius::same(2),
            Color32::WHITE,
        );
    }

    match tex {
        Some(tex) => {
            painter.image(tex.id(), rect, UV_FULL, Color32::WHITE);
        }
        None => {
            let color = if selected { ACCENT } else { hash_color(seed) };
            painter.circle_filled(center, rect.width() / 2.0, color);
            painter.text(
                center,
                Align2::CENTER_CENTER,
                initial_of(name),
                FontId::proportional(19.0),
                Color32::WHITE,
            );
        }
    }
    resp.on_hover_text(name.to_string())
}

// ------------------------------------------------------------- sidebar

fn status_color(status: &str) -> Color32 {
    match status {
        "online" => GREEN,
        "idle" => YELLOW,
        "dnd" => RED,
        _ => GRAY,
    }
}

fn status_label(status: &str) -> &'static str {
    match status {
        "online" => "Daring",
        "idle" => "Idle",
        "dnd" => "Jangan Ganggu",
        _ => "Luring",
    }
}

/// Avatar bulat dengan titik status di pojok kanan bawah.
fn avatar_with_status(
    ui: &mut egui::Ui,
    user: &User,
    textures: &HashMap<String, egui::TextureHandle>,
    status: &str,
    size: f32,
    ring: Color32,
) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), egui::Sense::hover());
    let painter = ui.painter();
    if let Some(tex) = user.avatar_url(64).as_ref().and_then(|u| textures.get(u)) {
        painter.image(tex.id(), rect, UV_FULL, Color32::WHITE);
    } else {
        painter.circle_filled(rect.center(), size / 2.0, hash_color(&user.id));
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            initial_of(user.display_name()),
            FontId::proportional(size * 0.42),
            Color32::WHITE,
        );
    }
    let dot = egui::Pos2::new(rect.right() - size * 0.12, rect.bottom() - size * 0.12);
    painter.circle_filled(dot, size * 0.18, ring);
    painter.circle_filled(dot, size * 0.14, status_color(status));
}

fn user_panel(app: &mut App, ui: &mut egui::Ui) {
    egui::TopBottomPanel::bottom("user_panel")
        .frame(
            egui::Frame::default()
                .fill(USER_PANEL_BG)
                .inner_margin(egui::Margin::symmetric(10, 8)),
        )
        .show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                // Avatar + nama + status
                if let Some(me) = app.me.clone() {
                    let status =
                        app.presences.get(&me.id).map(|s| s.as_str()).unwrap_or("online");
                    avatar_with_status(ui, &me, &app.textures, status, 32.0, USER_PANEL_BG);
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(me.display_name())
                                .strong()
                                .size(13.0)
                                .color(TEXT_MAIN),
                        );
                        ui.label(egui::RichText::new(status_label(status)).size(10.0).color(TEXT_DIM));
                    });
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    for icon in ["⚙", "🎧", "🎙"] {
                        let b = egui::Button::new(egui::RichText::new(icon).size(13.0))
                            .fill(Color32::TRANSPARENT);
                        ui.add_sized([26.0, 26.0], b);
                    }
                });
            });
        });
}

fn channel_sidebar(app: &mut App, ctx: &egui::Context) {
    let title = app
        .guilds
        .iter()
        .find(|g| app.selected_guild.as_deref() == Some(g.id.as_str()))
        .map(|g| g.name.clone())
        .unwrap_or_else(|| "Pesan Langsung".into());

    egui::SidePanel::left("channels")
        .exact_width(240.0)
        .resizable(false)
        .frame(egui::Frame::default().fill(SIDEBAR_BG))
        .show(ctx, |ui| {
            user_panel(app, ui);

            // Panel suara (muncul saat terhubung / menyambung), di atas panel user.
            if let Some((gid, _ch, name, connected)) = app.voice_panel.clone() {
                let guild_name = app
                    .guilds
                    .iter()
                    .find(|g| g.id == gid)
                    .map(|g| g.name.clone())
                    .unwrap_or_else(|| "server".into());
                let timer = app
                    .voice_start
                    .map(|t| format_voice_timer(t.elapsed()));
                egui::TopBottomPanel::bottom("voice_panel")
                    .frame(
                        egui::Frame::default()
                            .fill(USER_PANEL_BG)
                            .inner_margin(egui::Margin::symmetric(10, 8))
                            .corner_radius(egui::CornerRadius::same(6)),
                    )
                    .show_inside(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Ikon status + teks channel / server
                            ui.label(
                                egui::RichText::new("📶")
                                    .size(16.0)
                                    .color(if connected { GREEN } else { TEXT_DIM }),
                            );
                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new(if connected {
                                        "Voice Connected"
                                    } else {
                                        "Menyambung…"
                                    })
                                    .strong()
                                    .size(12.5)
                                    .color(if connected { GREEN } else { TEXT_DIM }),
                                );
                                ui.label(
                                    egui::RichText::new(format!("{name} / {guild_name}"))
                                        .size(10.5)
                                        .color(TEXT_DIM),
                                );
                            });
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if let Some(t) = &timer {
                                        ui.label(
                                            egui::RichText::new(t.as_str())
                                                .size(10.0)
                                                .color(TEXT_DIM),
                                        );
                                    }
                                    // Tombol putus (telepon merah)
                                    let b = egui::Button::new(
                                        egui::RichText::new("📞").size(13.0),
                                    )
                                    .fill(Color32::TRANSPARENT)
                                    .corner_radius(egui::CornerRadius::same(6));
                                    if ui
                                        .add_sized([26.0, 26.0], b)
                                        .on_hover_text("Putus voice")
                                        .clicked()
                                    {
                                        app.leave_voice();
                                    }
                                },
                            );
                        });
                    });
            }

            // Header nama server / beranda
            ui.add_sized(
                [ui.available_width(), 36.0],
                egui::Label::new(
                    egui::RichText::new(title).strong().size(15.0).color(TEXT_MAIN),
                ),
            );

            // Mode DM: kotak pencarian + item Teman + daftar DM
            if app.selected_guild.is_none() {
                egui::Frame::default()
                    .fill(RAIL_BG)
                    .corner_radius(egui::CornerRadius::same(4))
                    .inner_margin(egui::Margin::symmetric(8, 5))
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut app.filter_dms)
                                .hint_text("Cari atau mulai percakapan")
                                .frame(false)
                                .font(egui::TextStyle::Small),
                        );
                    });
                ui.add_space(4.0);

                let friends_sel = app.view == View::Friends;
                if channel_row(ui, "👥  Teman", None, friends_sel, false, None) {
                    app.set_friends_view();
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new("PESAN LANGSUNG")
                            .size(10.5)
                            .strong()
                            .color(TEXT_DIM),
                    );
                });
                ui.add_space(2.0);
            } else {
                ui.separator();
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                let mut action_text: Option<String> = None;
                let mut action_voice: Option<String> = None;
                let mut toggle: Option<String> = None;
                let mut current_category: Option<String> = None;

                let filter = app.filter_dms.trim().to_lowercase();
                let show_categories = filter.is_empty();

                for ch in &app.channels {
                    if !filter.is_empty()
                        && !ch.display_name().to_lowercase().contains(&filter)
                    {
                        continue;
                    }
                    if ch.is_category() {
                        if !show_categories {
                            continue;
                        }
                        let collapsed = app.collapsed.contains(&ch.id);
                        if category_row(ui, ch.name.as_deref().unwrap_or("kanal"), collapsed) {
                            toggle = Some(ch.id.clone());
                        }
                        current_category = Some(ch.id.clone());
                        continue;
                    }

                    // Channel voice: baris 🔊 + daftar yang berada di dalamnya.
                    if ch.kind == models::CHANNEL_VOICE {
                        if show_categories {
                            if let Some(cat) = &current_category {
                                if app.collapsed.contains(cat) {
                                    continue;
                                }
                            }
                        }
                        let active = app
                            .voice_panel
                            .as_ref()
                            .map(|(_, c, _, _)| c == &ch.id)
                            .unwrap_or(false);
                        let in_channel: Vec<String> = app
                            .voice_map
                            .values()
                            .flat_map(|m| m.iter())
                            .filter(|(_, chh)| chh.as_deref() == Some(ch.id.as_str()))
                            .map(|(uid, _)| uid.clone())
                            .collect();
                        let timer = active
                            .then(|| {
                                app.voice_start
                                    .map(|t| format_voice_timer(t.elapsed()))
                            })
                            .flatten();
                        if voice_row(ui, &ch.display_name(), active, timer) {
                            action_voice = Some(ch.id.clone());
                        }
                        for uid in &in_channel {
                            voice_member_row(ui, app, uid);
                        }
                        continue;
                    }

                    if !ch.is_textlike() {
                        continue;
                    }
                    if show_categories {
                        if let Some(cat) = &current_category {
                            if app.collapsed.contains(cat) {
                                continue;
                            }
                        }
                    }

                    let selected = app.view == View::Dms
                        && app.selected_channel.as_deref() == Some(ch.id.as_str());
                    let unread = app.unread.contains(&ch.id);
                    let tex = if ch.kind == models::CHANNEL_DM {
                        ch.recipients
                            .first()
                            .and_then(|u| u.avatar_url(64))
                            .as_ref()
                            .and_then(|u| app.textures.get(u))
                    } else {
                        None
                    };
                    let prefix = match ch.kind {
                        models::CHANNEL_TEXT | models::CHANNEL_ANNOUNCEMENT => "# ",
                        models::CHANNEL_GROUP_DM => "👥 ",
                        _ => "",
                    };
                    let preview: Option<(String, String)> = if ch.kind == models::CHANNEL_DM {
                        app.dm_previews.get(&ch.id).cloned()
                    } else {
                        None
                    };
                    if channel_row(
                        ui,
                        &format!("{prefix}{}", ch.display_name()),
                        preview.as_ref().map(|(a, c)| (a.as_str(), c.as_str())),
                        selected,
                        unread,
                        tex,
                    ) {
                        action_text = Some(ch.id.clone());
                    }
                }

                if let Some(id) = toggle {
                    app.toggle_category(id);
                }
                if let Some(id) = action_text {
                    if app.selected_guild.is_none() {
                        app.view = View::Dms;
                    }
                    app.select_channel(id);
                }
                if let Some(id) = action_voice {
                    app.join_voice(id);
                }
            });
        });
}

fn channel_row(
    ui: &mut egui::Ui,
    label: &str,
    preview: Option<(&str, &str)>,
    selected: bool,
    unread: bool,
    tex: Option<&egui::TextureHandle>,
) -> bool {
    let h = if preview.is_some() { 46.0 } else { 30.0 };
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), h), egui::Sense::click());
    let painter = ui.painter();

    if selected {
        painter.rect_filled(rect, egui::CornerRadius::same(4), SELECTED_BG);
    } else if ui.rect_contains_pointer(rect) {
        painter.rect_filled(rect, egui::CornerRadius::same(4), ROW_HOVER);
    }

    let x = rect.left() + 10.0;
    let mut text_x = x;
    if let Some(tex) = tex {
        let av = egui::Rect::from_min_size(
            egui::Pos2::new(x, rect.center().y - 13.0),
            Vec2::splat(26.0),
        );
        painter.image(tex.id(), av, UV_FULL, Color32::WHITE);
        text_x = x + 32.0;
    }
    let color = if selected || unread { TEXT_MAIN } else { TEXT_DIM };

    match preview {
        // Baris DM: nama di atas, potongan pesan terakhir di bawah.
        Some((author, content)) => {
            let y = rect.center().y;
            painter.text(
                egui::Pos2::new(text_x, y - 9.0),
                Align2::LEFT_CENTER,
                label,
                FontId::proportional(13.5),
                color,
            );
            let mut sub = format!("{author}: {content}");
            sub = one_line(&sub.replace('\n', " "), 30);
            painter.text(
                egui::Pos2::new(text_x, y + 10.0),
                Align2::LEFT_CENTER,
                sub,
                FontId::proportional(11.0),
                if unread { TEXT_MAIN } else { GRAY },
            );
        }
        None => {
            painter.text(
                egui::Pos2::new(text_x, rect.center().y),
                Align2::LEFT_CENTER,
                label,
                FontId::proportional(13.5),
                color,
            );
        }
    }
    if unread {
        painter.circle_filled(
            egui::Pos2::new(rect.right() - 10.0, rect.center().y),
            4.0,
            Color32::WHITE,
        );
    }
    resp.clicked()
}

/// Potong teks menjadi satu baris pendek.
fn one_line(s: &str, max_chars: usize) -> String {
    let s = s.trim();
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let cut: String = s.chars().take(max_chars).collect();
    format!("{cut}…")
}

fn category_row(ui: &mut egui::Ui, name: &str, collapsed: bool) -> bool {
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 24.0), egui::Sense::click());
    let arrow = if collapsed { "▸" } else { "▾" };
    ui.painter().text(
        egui::Pos2::new(rect.left() + 10.0, rect.center().y),
        Align2::LEFT_CENTER,
        format!("{arrow} {name}"),
        FontId::proportional(11.5),
        TEXT_DIM,
    );
    resp.clicked()
}

/// Baris channel voice (🔊 nama) — klik untuk join/leave.
// ---------------------------------------------------------------- voice page

/// Format durasi jadi "m:ss" / "h:mm:ss".
fn format_voice_timer(elapsed: std::time::Duration) -> String {
    let s = elapsed.as_secs();
    if s < 3600 {
        format!("{}:{:02}", s / 60, s % 60)
    } else {
        format!("{}:{:02}:{:02}", s / 3600, (s % 3600) / 60, s % 60)
    }
}

/// Halaman utama voice channel (ala aplikasi resmi): kartu peserta + bar kontrol.
fn voice_page(app: &mut App, ui: &mut egui::Ui) {
    let Some((gid, ch_id, name, _connected)) = app.voice_panel.clone() else {
        app.view = View::Dms; // sudah keluar voice; kembali ke DM
        return;
    };
    let participants: Vec<String> = app
        .voice_map
        .get(&gid)
        .map(|m| {
            m.iter()
                .filter(|(_, c)| c.as_deref() == Some(ch_id.as_str()))
                .map(|(u, _)| u.clone())
                .collect()
        })
        .unwrap_or_default();

    // Header ala channel: ikon speaker + nama channel.
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        ui.label(
            egui::RichText::new(format!("🔊 {name}"))
                .strong()
                .size(15.0)
                .color(TEXT_MAIN),
        );
        ui.separator();
        ui.label(
            egui::RichText::new(format!("Dengar saja • {} peserta", participants.len()))
                .size(12.0)
                .color(TEXT_DIM),
        );
    });
    ui.separator();

    // Bar kontrol di bawah; sisanya jadi kartu peserta.
    egui::TopBottomPanel::bottom("voice_controls")
        .frame(
            egui::Frame::default()
                .fill(CHAT_BG)
                .inner_margin(egui::Margin::symmetric(0, 10)),
        )
        .show_inside(ui, |ui| {
            ui.centered_and_justified(|ui| {
                ui.horizontal(|ui| {
                    // Mic: selalu mati di Phase V1 (dengar saja).
                    voice_ctl_button(ui, "🎙", false, true, "Mic: belum didukung (Phase V2)");
                    // Deafen: bisa ditoggle.
                    if voice_ctl_button(ui, "🎧", app.self_deaf, false, "Deafen (tolak semua audio)") {
                        app.toggle_deaf();
                    }
                    // Layar & video: belum didukung.
                    voice_ctl_button(ui, "🖥", false, true, "Bagikan layar: belum didukung");
                    voice_ctl_button(ui, "📹", false, true, "Video: belum didukung");
                    // Putus (merah).
                    let b = egui::Button::new(egui::RichText::new("📞").size(17.0))
                        .fill(RED)
                        .corner_radius(egui::CornerRadius::same(20));
                    if ui
                        .add_sized([48.0, 40.0], b)
                        .on_hover_text("Putus voice")
                        .clicked()
                    {
                        app.leave_voice();
                    }
                });
            });
        });

    egui::CentralPanel::default()
        .frame(egui::Frame::default().fill(CHAT_BG))
        .show_inside(ui, |ui| {
            if participants.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        egui::RichText::new("Belum ada yang di voice — tunggu orang masuk…")
                            .size(13.0)
                            .color(TEXT_DIM),
                    );
                });
                return;
            }

            // Petak terpisah per peserta: setiap user satu kotak sendiri
            // (tidak menyambung satu sama lain), grid di tengah.
            let av_size = 64.0_f32;
            let tile_w = 168.0_f32;
            let tile_h = 130.0_f32;
            let gap = 12.0;
            let avail = ui.available_width();
            let cols = ((avail - 20.0) / (tile_w + gap)).max(1.0).floor() as usize;
            let rows: Vec<&[String]> = participants.chunks(cols.max(1)).collect();

            // Tengah vertikal: sisakan setengah ruang kosong di atas.
            let total_h = rows.len() as f32 * (tile_h + gap);
            let spare = (ui.available_height() - total_h).max(0.0) / 2.0;
            ui.add_space(spare);

            for row in rows {
                let row_w = row.len() as f32 * tile_w + (row.len().saturating_sub(1)) as f32 * gap;
                let left_pad = ((avail - row_w) / 2.0).max(0.0);
                ui.horizontal(|ui| {
                    ui.add_space(left_pad);
                    for uid in row {
                        let name = app.resolve_name(uid);
                        let is_me = app.me.as_ref().map(|m| m.id.as_str()) == Some(uid.as_str());
                        egui::Frame::default()
                            .fill(Color32::from_rgb(0x2b, 0x2d, 0x31))
                            .corner_radius(egui::CornerRadius::same(10))
                            .inner_margin(egui::Margin::same(10))
                            .show(ui, |ui| {
                                ui.set_width(tile_w - 20.0);
                                ui.vertical_centered(|ui| {
                                    ui.add_space(6.0);
                                    let tex = member_avatar_tex(app, uid);
                                    let (rect, _) = ui.allocate_exact_size(
                                        Vec2::splat(av_size),
                                        egui::Sense::hover(),
                                    );
                                    let painter = ui.painter();
                                    match &tex {
                                        Some(t) => {
                                            painter.image(t.id(), rect, UV_FULL, Color32::WHITE);
                                        }
                                        None => {
                                            painter.circle_filled(
                                                rect.center(),
                                                av_size / 2.0,
                                                hash_color(uid),
                                            );
                                            painter.text(
                                                rect.center(),
                                                Align2::CENTER_CENTER,
                                                initial_of(&name),
                                                FontId::proportional(av_size * 0.42),
                                                Color32::WHITE,
                                            );
                                        }
                                    }
                                    ui.add_space(6.0);
                                    // Kita = dengar saja: badge hijau di bawah nama.
                                    ui.label(
                                        egui::RichText::new(one_line(&name, 14))
                                            .size(12.0)
                                            .color(TEXT_MAIN),
                                    );
                                    if is_me {
                                        ui.label(
                                            egui::RichText::new("🎧 dengar saja")
                                                .size(9.5)
                                                .color(GREEN),
                                        );
                                    } else {
                                        ui.add_space(13.0);
                                    }
                                });
                            });
                        ui.add_space(gap);
                    }
                });
                ui.add_space(gap);
            }
        });
}

/// Tombol bulat di bar kontrol voice. `disabled` = abu-abu, tak bisa diklik.
fn voice_ctl_button(
    ui: &mut egui::Ui,
    icon: &str,
    active: bool,
    disabled: bool,
    tooltip: &str,
) -> bool {
    let (fill, fg) = if disabled {
        (Color32::from_rgb(0x2b, 0x2d, 0x31), TEXT_DIM)
    } else if active {
        (Color32::from_rgb(0x5d, 0x24, 0x26), RED) // merah gelap saat menyala (mute/deafen)
    } else {
        (Color32::from_rgb(0x4a, 0x4d, 0x54), TEXT_MAIN)
    };
    let b = egui::Button::new(egui::RichText::new(icon).size(16.0).color(fg))
        .fill(fill)
        .corner_radius(egui::CornerRadius::same(20));
    let resp = ui.add_sized([48.0, 40.0], b).on_hover_text(tooltip);
    !disabled && resp.clicked()
}

/// Avatar texture seorang user dari cache (DM recipients / guild members).
fn member_avatar_tex(app: &App, user_id: &str) -> Option<egui::TextureHandle> {
    app.channels
        .iter()
        .filter_map(|c| c.recipients.iter().find(|u| u.id == user_id))
        .chain(app.guild_members.values().flatten().filter(|u| u.id == user_id))
        .find_map(|u| u.avatar_url(64).and_then(|a| app.textures.get(&a).cloned()))
}

fn voice_row(ui: &mut egui::Ui, name: &str, active: bool, timer: Option<String>) -> bool {
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 26.0), egui::Sense::click());
    let painter = ui.painter();
    if active {
        painter.rect_filled(rect, egui::CornerRadius::same(4), SELECTED_BG);
    } else if ui.rect_contains_pointer(rect) {
        painter.rect_filled(rect, egui::CornerRadius::same(4), ROW_HOVER);
    }
    let icon = if active { "🔊" } else { "🔊" };
    let t_col = if active { TEXT_MAIN } else { TEXT_DIM };
    painter.text(
        egui::Pos2::new(rect.left() + 14.0, rect.center().y),
        Align2::LEFT_CENTER,
        format!("{icon} {name}"),
        FontId::proportional(12.5),
        t_col,
    );
    if let Some(t) = timer {
        painter.text(
            egui::Pos2::new(rect.right() - 10.0, rect.center().y),
            Align2::RIGHT_CENTER,
            t,
            FontId::monospace(10.5),
            GREEN,
        );
    }
    resp.clicked()
}

/// Anggota yang sedang berada di channel voice (avatar + nama).
fn voice_member_row(ui: &mut egui::Ui, app: &App, user_id: &str) {
    let name = app.resolve_name(user_id);
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 22.0), egui::Sense::hover());
    let painter = ui.painter();
    let av = egui::Rect::from_min_size(
        egui::Pos2::new(rect.left() + 44.0, rect.center().y - 8.0),
        Vec2::splat(16.0),
    );
    // Avatar asli jika tersedia.
    let tex = app
        .channels
        .iter()
        .filter_map(|c| c.recipients.iter().find(|u| u.id == user_id))
        .chain(app.guild_members.values().flatten().filter(|u| u.id == user_id))
        .find_map(|u| u.avatar_url(64).and_then(|a| app.textures.get(&a).cloned()));
    match &tex {
        Some(tex) => {
            painter.image(tex.id(), av, UV_FULL, Color32::WHITE);
        }
        None => {
            painter.circle_filled(av.center(), 8.0, hash_color(user_id));
            painter.text(
                av.center(),
                Align2::CENTER_CENTER,
                initial_of(&name),
                FontId::proportional(8.0),
                Color32::WHITE,
            );
        }
    }
    painter.text(
        egui::Pos2::new(av.right() + 8.0, rect.center().y),
        Align2::LEFT_CENTER,
        one_line(&name, 16),
        FontId::proportional(12.0),
        TEXT_DIM,
    );
}

// ------------------------------------------------------- panel anggota

/// Panel daftar anggota di kanan (hanya mode server), ala Serein/Discord.
fn member_panel(app: &App, ctx: &egui::Context) {
    if app.view != View::Guild {
        return;
    }
    let Some(gid) = app.selected_guild.as_deref() else { return };
    let Some(members) = app.guild_members.get(gid) else { return };
    if members.is_empty() {
        return;
    }

    egui::SidePanel::right("member_list")
        .exact_width(200.0)
        .resizable(false)
        .frame(egui::Frame::default().fill(CHAT_BG))
        .show(ctx, |ui| {
            ui.add_space(10.0);
            let online: Vec<&User> =
                members.iter().filter(|u| app.is_online(&u.id)).collect();
            ui.label(
                egui::RichText::new(format!("DARING — {}", online.len()))
                    .size(10.5)
                    .strong()
                    .color(TEXT_DIM),
            );
            ui.add_space(4.0);
            egui::ScrollArea::vertical().show(ui, |ui| {
                for u in &online {
                    member_row(ui, app, u);
                }
                let offline: Vec<&User> =
                    members.iter().filter(|u| !app.is_online(&u.id)).collect();
                if !offline.is_empty() {
                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new(format!("LURING — {}", offline.len()))
                            .size(10.5)
                            .strong()
                            .color(TEXT_DIM),
                    );
                    ui.add_space(4.0);
                    for u in &offline {
                        member_row(ui, app, u);
                    }
                }
                ui.add_space(16.0);
            });
        });
}

fn member_row(ui: &mut egui::Ui, app: &App, user: &User) {
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 32.0), egui::Sense::hover());
    let painter = ui.painter();
    let online = app.is_online(&user.id);
    if ui.rect_contains_pointer(rect) {
        painter.rect_filled(rect, egui::CornerRadius::same(4), ROW_HOVER);
    }

    let av = egui::Rect::from_min_size(
        egui::Pos2::new(rect.left() + 8.0, rect.center().y - 12.0),
        Vec2::splat(24.0),
    );
    if let Some(tex) = user.avatar_url(64).as_ref().and_then(|u| app.textures.get(u)) {
        painter.image(tex.id(), av, UV_FULL, Color32::WHITE);
    } else {
        painter.circle_filled(av.center(), 12.0, hash_color(&user.id));
        painter.text(
            av.center(),
            Align2::CENTER_CENTER,
            initial_of(user.display_name()),
            FontId::proportional(11.0),
            Color32::WHITE,
        );
    }
    let dot = egui::Pos2::new(av.right() - 1.0, av.bottom() - 1.0);
    painter.circle_filled(dot, 5.0, CHAT_BG);
    painter.circle_filled(dot, 3.8, status_color(if online {
        app.presences.get(&user.id).map(|s| s.as_str()).unwrap_or("online")
    } else {
        "offline"
    }));

    let name_color = if online { TEXT_MAIN } else { TEXT_DIM };
    painter.text(
        egui::Pos2::new(av.right() + 8.0, rect.center().y),
        Align2::LEFT_CENTER,
        one_line(user.display_name(), 14),
        FontId::proportional(13.0),
        name_color,
    );
}

// ---------------------------------------------------------------- chat

fn chat_panel(app: &mut App, ctx: &egui::Context) {
    if app.view == View::Friends {
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(CHAT_BG))
            .show(ctx, |ui| friends_page(app, ui));
        return;
    }

    if app.view == View::Voice {
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(CHAT_BG))
            .show(ctx, |ui| voice_page(app, ui));
        return;
    }

    let Some(channel_id) = app.selected_channel.clone() else {
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(CHAT_BG))
            .show(ctx, |ui| {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        egui::RichText::new("Pilih channel untuk mulai chat 👋")
                            .size(16.0)
                            .color(TEXT_DIM),
                    );
                });
            });
        return;
    };

    let (ch_name, ch_topic, ch_kind) = match app.channel_by_id(&channel_id) {
        Some(ch) => (ch.display_name(), ch.topic.clone().unwrap_or_default(), ch.kind),
        None => ("channel".into(), String::new(), models::CHANNEL_TEXT),
    };
    let is_dm_ch = matches!(
        ch_kind,
        models::CHANNEL_DM | models::CHANNEL_GROUP_DM
    );

    // Peta id -> nama untuk mention & channel-link
    let mention_names: HashMap<String, String> = app
        .messages
        .iter()
        .flat_map(|m| m.mentions.iter().map(|u| (u.id.clone(), u.display_name().to_string())))
        .collect();
    let channel_names: HashMap<String, String> = app
        .channels
        .iter()
        .map(|c| (c.id.clone(), c.display_name()))
        .collect();
    let me_id = app.me.as_ref().map(|u| u.id.clone()).unwrap_or_default();

    egui::CentralPanel::default()
        .frame(egui::Frame::default().fill(CHAT_BG))
        .show(ctx, |ui| {
            // Header channel (DM tanpa tanda pagar, ala Discord)
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                let title = if is_dm_ch {
                    ch_name.clone()
                } else {
                    format!("# {ch_name}")
                };
                ui.label(
                    egui::RichText::new(title).strong().size(15.0).color(TEXT_MAIN),
                );
                if !ch_topic.is_empty() {
                    ui.separator();
                    ui.label(egui::RichText::new(&ch_topic).size(12.0).color(TEXT_DIM));
                }
            });
            ui.separator();
            ui.add_space(8.0);

            // Daftar pesan
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    ui.add_space(4.0);

                    // Tombol muat pesan lama (pagination, seperti Serein).
                    if !app.messages.is_empty() && app.has_more {
                        ui.add_space(4.0);
                        let label = if app.older_loading {
                            "memuat pesan lama…"
                        } else {
                            "⬆  Muat pesan lama"
                        };
                        let btn = egui::Button::new(
                            egui::RichText::new(label).size(11.5).color(TEXT_DIM),
                        )
                        .fill(Color32::TRANSPARENT);
                        if ui.add_sized([160.0, 24.0], btn).clicked() {
                            app.load_older();
                        }
                    }

                    // Tampilan kosong: error akses / awal percakapan.
                    if app.messages.is_empty() {
                        if let Some(err) = app.load_errors.get(&channel_id) {
                            ui.add_space(60.0);
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    egui::RichText::new("🚫").size(42.0),
                                );
                                ui.add_space(10.0);
                                ui.label(
                                    egui::RichText::new(err)
                                        .size(16.0)
                                        .strong()
                                        .color(TEXT_MAIN),
                                );
                                ui.add_space(4.0);
                                ui.label(
                                    egui::RichText::new(
                                        "Kamu tidak diizinkan membaca channel ini (403).",
                                    )
                                    .size(12.5)
                                    .color(TEXT_DIM),
                                );
                            });
                        } else {
                            empty_state(ui, app, &channel_id, &ch_name);
                        }
                    }

                    let mut prev_author: Option<String> = None;
                    let mut prev_date: Option<String> = None;
                    for msg in &app.messages {
                        let date: String = msg.timestamp.chars().take(10).collect();
                        let date_changed = prev_date.as_deref() != Some(date.as_str());
                        let with_header =
                            date_changed || prev_author.as_deref() != Some(&msg.author.id);
                        if date_changed && !msg.timestamp.is_empty() {
                            date_divider(ui, &msg.timestamp);
                        }
                        message_row(ui, msg, with_header,
                            &mention_names, &channel_names, &app.textures, &me_id);
                        prev_author = Some(msg.author.id.clone());
                        prev_date = Some(date);
                    }
                    ui.add_space(12.0);
                });

            ui.add_space(6.0);

            // Indikator "sedang mengetik…" ala Discord
            let typing_now = app
                .typing
                .get(&channel_id)
                .filter(|(_, t)| t.elapsed() < std::time::Duration::from_secs(8))
                .map(|(name, _)| name.clone());
            if let Some(name) = &typing_now {
                ui.horizontal(|ui| {
                    ui.add_space(26.0);
                    ui.label(
                        egui::RichText::new(format!("{name} sedang mengetik…"))
                            .size(11.0)
                            .italics()
                            .color(TEXT_DIM),
                    );
                });
                ui.add_space(2.0);
            }

            // Composer dalam frame rounded, ikon ala Discord
            ui.horizontal(|ui| {
                ui.add_space(16.0);
                egui::Frame::default()
                    .fill(INPUT_BG)
                    .corner_radius(egui::CornerRadius::same(8))
                    .inner_margin(egui::Margin::symmetric(8, 4))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Tombol + (lampirkan berkas; dekoratif dulu)
                            icon_button(ui, "+", 18.0, TEXT_DIM);

                            let width = ui.available_width() - 116.0;
                            let hint = if is_dm_ch {
                                format!("Kirim pesan ke @{ch_name}")
                            } else {
                                format!("Kirim pesan ke #{ch_name}")
                            };
                            let te = egui::TextEdit::singleline(&mut app.composer)
                                .hint_text(hint)
                                .frame(false)
                                .desired_width(width);
                            let resp = ui.add(te);
                            let enter = resp.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter));
                            if enter {
                                app.send();
                                resp.request_focus();
                            }

                            icon_button(ui, "GIF", 11.0, TEXT_DIM);
                            icon_button(ui, "😊", 14.0, TEXT_DIM);

                            // Tombol kirim bulat aksen
                            let (srect, sresp) = ui.allocate_exact_size(
                                Vec2::splat(30.0),
                                egui::Sense::click(),
                            );
                            let fill = if !app.composer.trim().is_empty() {
                                ACCENT
                            } else {
                                Color32::from_rgb(0x4e, 0x50, 0x58)
                            };
                            ui.painter().circle_filled(srect.center(), 15.0, fill);
                            ui.painter().text(
                                srect.center(),
                                Align2::CENTER_CENTER,
                                "➤",
                                FontId::proportional(13.0),
                                Color32::WHITE,
                            );
                            if sresp.clicked() || enter {
                                app.send();
                            }
                        });
                    });
                ui.add_space(16.0);
            });
            ui.add_space(10.0);
        });
}

/// Tombol ikon transparan kecil (dekoratif).
fn icon_button(ui: &mut egui::Ui, icon: &str, size: f32, color: Color32) -> egui::Response {
    let (rect, resp) =
        ui.allocate_exact_size(Vec2::new(28.0, 26.0), egui::Sense::click());
    if ui.rect_contains_pointer(rect) {
        ui.painter().rect_filled(
            rect,
            egui::CornerRadius::same(4),
            Color32::from_rgba_unmultiplied(255, 255, 255, 12),
        );
    }
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        icon,
        FontId::proportional(size),
        color,
    );
    resp
}

// ------------------------------------------------------- halaman teman

fn friends_page(app: &mut App, ui: &mut egui::Ui) {
    // Header
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        ui.add_space(8.0);
        ui.label(
            egui::RichText::new("👥  Teman").strong().size(16.0).color(TEXT_MAIN),
        );
        ui.separator();
        if ui
            .selectable_label(
                !app.friends_show_all,
                egui::RichText::new("Daring").size(13.0),
            )
            .clicked()
        {
            app.friends_show_all = false;
        }
        if ui
            .selectable_label(
                app.friends_show_all,
                egui::RichText::new("Semua").size(13.0),
            )
            .clicked()
        {
            app.friends_show_all = true;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(12.0);
            let btn = egui::Button::new(egui::RichText::new("Tambah Teman").size(12.5))
                .fill(ACCENT);
            ui.add(btn);
        });
    });
    ui.separator();
    ui.add_space(10.0);

    // Pencarian
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        egui::Frame::default()
            .fill(RAIL_BG)
            .corner_radius(egui::CornerRadius::same(6))
            .inner_margin(egui::Margin::symmetric(10, 6))
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut app.filter_friends)
                        .hint_text("Cari")
                        .frame(false)
                        .desired_width(300.0),
                );
            });
    });
    ui.add_space(10.0);

    let filter = app.filter_friends.trim().to_lowercase();
    let is_online = |app: &App, id: &str| -> bool {
        matches!(
            app.presences.get(id).map(|s| s.as_str()),
            Some("online") | Some("idle") | Some("dnd")
        )
    };

    let friends: Vec<&User> = app
        .friends
        .iter()
        .filter(|f| {
            if !filter.is_empty()
                && !f.display_name().to_lowercase().contains(&filter)
            {
                return false;
            }
            if !app.friends_show_all && !is_online(app, &f.id) {
                return false;
            }
            true
        })
        .collect();

    if !app.friends_show_all {
        ui.label(
            egui::RichText::new(format!("Daring — {}", friends.len()))
                .size(11.5)
                .strong()
                .color(TEXT_DIM),
        );
    } else {
        ui.label(
            egui::RichText::new(format!("Semua teman — {}", friends.len()))
                .size(11.5)
                .strong()
                .color(TEXT_DIM),
        );
    }
    ui.add_space(4.0);

    egui::ScrollArea::vertical().show(ui, |ui| {
        for f in friends {
            let status =
                app.presences.get(&f.id).map(|s| s.as_str()).unwrap_or("offline");
            let (rect, resp) = ui.allocate_exact_size(
                Vec2::new(ui.available_width(), 44.0),
                egui::Sense::click(),
            );
            let painter = ui.painter();
            if ui.rect_contains_pointer(rect) {
                painter.rect_filled(rect, egui::CornerRadius::same(4), ROW_HOVER);
            }
            let av_rect = egui::Rect::from_min_size(
                egui::Pos2::new(rect.left() + 16.0, rect.center().y - 16.0),
                Vec2::splat(32.0),
            );
            if let Some(tex) =
                f.avatar_url(64).as_ref().and_then(|u| app.textures.get(u))
            {
                painter.image(tex.id(), av_rect, UV_FULL, Color32::WHITE);
            } else {
                painter.circle_filled(av_rect.center(), 16.0, hash_color(&f.id));
                painter.text(
                    av_rect.center(),
                    Align2::CENTER_CENTER,
                    initial_of(f.display_name()),
                    FontId::proportional(13.0),
                    Color32::WHITE,
                );
            }
            let dot = egui::Pos2::new(av_rect.right() - 3.0, av_rect.bottom() - 3.0);
            painter.circle_filled(dot, 5.5, CHAT_BG);
            painter.circle_filled(dot, 4.0, status_color(status));
            painter.text(
                egui::Pos2::new(av_rect.right() + 12.0, rect.center().y),
                Align2::LEFT_CENTER,
                f.display_name(),
                FontId::proportional(14.0),
                TEXT_MAIN,
            );
            let _ = resp;
        }
        ui.add_space(16.0);
    });
}

// ------------------------------------------------------- tampilan kosong

/// "Ini adalah awal dari percakapanmu…" ala Discord.
fn empty_state(ui: &mut egui::Ui, app: &App, channel_id: &str, ch_name: &str) {
    let ch = app.channel_by_id(channel_id);
    let is_dm = matches!(ch.map(|c| c.kind), Some(models::CHANNEL_DM));

    ui.add_space(40.0);
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(80.0), egui::Sense::hover());
        let painter = ui.painter();
        let r = rect.center();
        // Cincin glow dekoratif ala Discord.
        painter.circle_filled(r, 46.0, ACCENT.gamma_multiply(0.12));
        painter.circle_filled(r, 40.0, ACCENT.gamma_multiply(0.25));
        match ch.and_then(|c| c.recipients.first().cloned()) {
            Some(user) if is_dm => {
                if let Some(tex) =
                    user.avatar_url(128).as_ref().and_then(|u| app.textures.get(u))
                {
                    painter.image(tex.id(), rect, UV_FULL, Color32::WHITE);
                } else {
                    painter.circle_filled(r, 40.0, hash_color(&user.id));
                    painter.text(
                        r,
                        Align2::CENTER_CENTER,
                        initial_of(user.display_name()),
                        FontId::proportional(32.0),
                        Color32::WHITE,
                    );
                }
            }
            _ => {
                painter.circle_filled(r, 40.0, ACCENT);
                painter.text(
                    r,
                    Align2::CENTER_CENTER,
                    "#",
                    FontId::proportional(34.0),
                    Color32::WHITE,
                );
            }
        }
    });
    ui.add_space(14.0);
    ui.label(
        egui::RichText::new(if is_dm {
            format!("Ini adalah awal dari percakapanmu dengan {ch_name}.")
        } else {
            format!("Selamat datang di #{ch_name}!")
        })
        .strong()
        .size(28.0)
        .color(TEXT_MAIN),
    );
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new(if is_dm {
            format!("{ch_name} di sini — mulai percakapan dengan sapaan hangat! 🎉")
        } else {
            format!("Ini awal channel #{ch_name}.")
        })
        .size(14.0)
        .color(TEXT_DIM),
    );
    ui.add_space(20.0);
}

// ------------------------------------------------------------- reaksi

/// Deretan pill reaksi di bawah pesan (emoji + jumlah).
fn reactions_row(
    ui: &mut egui::Ui,
    msg: &Message,
    indent: f32,
    textures: &HashMap<String, egui::TextureHandle>,
) {
    if msg.reactions.is_empty() {
        return;
    }
    ui.horizontal(|ui| {
        ui.add_space(indent);
        for r in &msg.reactions {
            let fill = if r.me {
                Color32::from_rgb(0x3b, 0x44, 0x74)
            } else {
                Color32::from_rgb(0x2b, 0x2d, 0x31)
            };
            egui::Frame::default()
                .fill(fill)
                .corner_radius(egui::CornerRadius::same(8))
                .inner_margin(egui::Margin::symmetric(7, 3))
                .show(ui, |ui| {
                    // Emoji kustom ditampilkan sebagai gambar dari CDN.
                    let tex = r
                        .emoji
                        .image_url()
                        .as_ref()
                        .and_then(|u| textures.get(u));
                    match tex {
                        Some(tex) => {
                            let (rect, _) = ui.allocate_exact_size(
                                Vec2::splat(16.0),
                                egui::Sense::hover(),
                            );
                            ui.painter()
                                .image(tex.id(), rect, UV_FULL, Color32::WHITE);
                            ui.label(
                                egui::RichText::new(format!("{}", r.count))
                                    .size(12.5)
                                    .color(if r.me { MENTION } else { TEXT_MAIN }),
                            );
                        }
                        None => {
                            let label = format!("{} {}", r.emoji.label(), r.count);
                            ui.label(
                                egui::RichText::new(&label)
                                    .size(12.5)
                                    .color(if r.me { MENTION } else { TEXT_MAIN }),
                            );
                        }
                    }
                });
        }
    });
    ui.add_space(2.0);
}

// --------------------------------------------------------- widget pesan

fn date_divider(ui: &mut egui::Ui, ts: &str) {
    const MONTHS: [&str; 12] = [
        "Januari", "Februari", "Maret", "April", "Mei", "Juni",
        "Juli", "Agustus", "September", "Oktober", "November", "Desember",
    ];
    let label = DateTime::parse_from_rfc3339(ts)
        .ok()
        .map(|dt| {
            let l = dt.with_timezone(&Local);
            format!("{} {} {}", l.day(), MONTHS[(l.month0() as usize) % 12], l.year())
        })
        .unwrap_or_default();
    ui.add_space(6.0);
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 18.0), egui::Sense::hover());
    let painter = ui.painter();
    let y = rect.center().y;
    let stroke = egui::Stroke::new(1.0_f32, Color32::from_rgb(0x4e, 0x50, 0x58));
    // Lebar teks diukur agar garis kiri-kanan simetris.
    let galley = painter.layout_no_wrap(
        label.clone(),
        FontId::proportional(11.0),
        TEXT_DIM,
    );
    let text_half = galley.size().x / 2.0 + 10.0;
    painter.galley(
        egui::Pos2::new(rect.center().x - galley.size().x / 2.0, y - galley.size().y / 2.0),
        galley,
        TEXT_DIM,
    );
    painter.line_segment(
        [
            egui::Pos2::new(rect.left() + 16.0, y),
            egui::Pos2::new(rect.center().x - text_half, y),
        ],
        stroke,
    );
    painter.line_segment(
        [
            egui::Pos2::new(rect.center().x + text_half, y),
            egui::Pos2::new(rect.right() - 16.0, y),
        ],
        stroke,
    );
    ui.add_space(6.0);
}

fn message_row(
    ui: &mut egui::Ui,
    msg: &Message,
    with_header: bool,
    mention_names: &HashMap<String, String>,
    channel_names: &HashMap<String, String>,
    textures: &HashMap<String, egui::TextureHandle>,
    me_id: &str,
) {
    let top = ui.cursor().top();

    if with_header {
        ui.add_space(10.0);
        // Bar balasan kecil di atas header (ala Discord).
        if let Some(r) = &msg.referenced_message {
            ui.horizontal(|ui| {
                ui.add_space(58.0);
                let snippet = if r.content.is_empty() {
                    "📎 lampiran".to_string()
                } else {
                    one_line(&r.content.replace('\n', " "), 48)
                };
                ui.label(
                    egui::RichText::new(format!(
                        "↳ {} — {snippet}",
                        r.author.display_name()
                    ))
                    .size(11.0)
                    .color(TEXT_DIM),
                );
            });
            ui.add_space(1.0);
        }
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            avatar_widget(ui, &msg.author, textures);
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(msg.author.display_name())
                            .strong()
                            .size(14.0)
                            .color(hash_color(&msg.author.id)),
                    );
                    if let Some(t) = fmt_time(&msg.timestamp) {
                        ui.label(egui::RichText::new(t).size(11.0).color(TEXT_DIM));
                    }
                    if msg.edited_timestamp.is_some() {
                        ui.label(
                            egui::RichText::new("(diedit)").size(10.0).color(TEXT_DIM),
                        );
                    }
                });
                render_markdown(ui, &msg.content, mention_names, channel_names);
                for att in &msg.attachments {
                    attachment_widget(ui, att, textures);
                }
            });
        });
    } else {
        ui.horizontal(|ui| {
            ui.add_space(62.0);
            render_markdown(ui, &msg.content, mention_names, channel_names);
            for att in &msg.attachments {
                attachment_widget(ui, att, textures);
            }
        });
    }
    reactions_row(
        ui,
        msg,
        if with_header { 58.0 } else { 62.0 },
        textures,
    );
    ui.add_space(2.0);

    // Latar baris: kuning jika menyebut kita, putih tipis saat hover.
    let bottom = ui.cursor().top();
    let hover_rect = egui::Rect::from_min_max(
        egui::Pos2::new(ui.max_rect().left(), top),
        egui::Pos2::new(ui.max_rect().right(), bottom),
    );
    let mentions_me =
        !me_id.is_empty() && msg.mentions.iter().any(|u| u.id == me_id);
    if mentions_me {
        ui.painter().rect_filled(
            hover_rect,
            egui::CornerRadius::same(0),
            Color32::from_rgba_unmultiplied(250, 166, 26, 22),
        );
    } else if ui.rect_contains_pointer(hover_rect) {
        ui.painter().rect_filled(
            hover_rect,
            egui::CornerRadius::same(0),
            Color32::from_rgba_unmultiplied(255, 255, 255, 8),
        );
    }
}

fn avatar_widget(ui: &mut egui::Ui, user: &User, textures: &HashMap<String, egui::TextureHandle>) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(36.0), egui::Sense::hover());
    let painter = ui.painter();
    if let Some(tex) = user.avatar_url(64).as_ref().and_then(|u| textures.get(u)) {
        painter.image(tex.id(), rect, UV_FULL, Color32::WHITE);
    } else {
        painter.circle_filled(rect.center(), 18.0, hash_color(&user.id));
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            initial_of(user.display_name()),
            FontId::proportional(15.0),
            Color32::WHITE,
        );
    }
}

fn attachment_widget(
    ui: &mut egui::Ui,
    att: &Attachment,
    textures: &HashMap<String, egui::TextureHandle>,
) {
    ui.add_space(4.0);
    if att.is_image() {
        if let Some(tex) = textures.get(&att.url) {
            let size = tex.size_vec2();
            let scale = (400.0 / size.x).min(300.0 / size.y).min(1.0).max(0.0);
            let fit = size * scale;
            if fit.x > 1.0 && fit.y > 1.0 {
                let (rect, _resp) =
                    ui.allocate_exact_size(fit, egui::Sense::click());
                ui.painter().image(tex.id(), rect, UV_FULL, Color32::WHITE);
                ui.add_space(4.0);
                return;
            }
        }
        // Belum termuat
        ui.label(
            egui::RichText::new(format!("🖼  memuat {}…", att.filename))
                .size(11.0)
                .color(TEXT_DIM),
        );
    } else {
        egui::Frame::default()
            .fill(SIDEBAR_BG)
            .corner_radius(egui::CornerRadius::same(6))
            .inner_margin(egui::Margin::symmetric(10, 6))
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(format!("📎  {}", att.filename))
                        .size(12.5)
                        .color(LINK),
                );
            });
    }
}

// ------------------------------------------------------------ markdown

#[derive(Debug)]
enum Span {
    Text(String),
    Bold(String),
    Italic(String),
    Underline(String),
    Strike(String),
    Code(String),
    Mention(String),
    ChannelLink(String),
    Url(String),
    Emoji(String),
}

fn parse_inline(
    line: &str,
    mentions: &HashMap<String, String>,
    channels: &HashMap<String, String>,
) -> Vec<Span> {
    let mut spans: Vec<Span> = Vec::new();
    let mut plain = String::new();
    let bytes = line.as_bytes();
    let mut i = 0usize;

    fn flush(plain: &mut String, spans: &mut Vec<Span>) {
        if !plain.is_empty() {
            spans.push(Span::Text(std::mem::take(plain)));
        }
    }

    while i < bytes.len() {
        let rest = &line[i..];

        // URL
        if rest.starts_with("http://") || rest.starts_with("https://") {
            let end = rest
                .find(|c: char| c.is_whitespace() || c == '<')
                .unwrap_or(rest.len());
            let url = rest[..end].trim_end_matches(['.', ',', ';', '!', '?', ')']);
            if url.len() > 8 {
                flush(&mut plain, &mut spans);
                spans.push(Span::Url(url.to_string()));
                i += url.len();
                continue;
            }
        }

        // Tag <...>: mention, channel, emoji kustom
        if bytes[i] == b'<' {
            if let Some(gt) = line[i..].find('>') {
                let inner = &line[i + 1..i + gt];
                let id = inner.strip_prefix("@!").or_else(|| inner.strip_prefix('@'));
                if let Some(id) = id {
                    let name =
                        mentions.get(id).cloned().unwrap_or_else(|| "user".into());
                    flush(&mut plain, &mut spans);
                    spans.push(Span::Mention(format!("@{name}")));
                    i += gt + 1;
                    continue;
                }
                if let Some(id) = inner.strip_prefix('#') {
                    let name = channels
                        .get(id)
                        .cloned()
                        .unwrap_or_else(|| "channel".into());
                    flush(&mut plain, &mut spans);
                    spans.push(Span::ChannelLink(format!("#{name}")));
                    i += gt + 1;
                    continue;
                }
                let em = inner.strip_prefix('a').and_then(|r| r.strip_prefix(':'));
                if let Some(rest2) = em.or_else(|| inner.strip_prefix(':')) {
                    if let Some(colon) = rest2.find(':') {
                        flush(&mut plain, &mut spans);
                        spans.push(Span::Emoji(format!(":{}:", &rest2[..colon])));
                        i += gt + 1;
                        continue;
                    }
                }
            }
        }

        // Marker teks (urutan penting: ** sebelum *, __ sebelum _)
        if let Some(close) = rest.strip_prefix("**").and_then(|r| r.find("**")) {
            if close > 0 {
                flush(&mut plain, &mut spans);
                spans.push(Span::Bold(rest[2..2 + close].to_string()));
                i += 2 + close + 2;
                continue;
            }
        }
        if let Some(close) = rest.strip_prefix("__").and_then(|r| r.find("__")) {
            if close > 0 {
                flush(&mut plain, &mut spans);
                spans.push(Span::Underline(rest[2..2 + close].to_string()));
                i += 2 + close + 2;
                continue;
            }
        }
        if let Some(close) = rest.strip_prefix("~~").and_then(|r| r.find("~~")) {
            if close > 0 {
                flush(&mut plain, &mut spans);
                spans.push(Span::Strike(rest[2..2 + close].to_string()));
                i += 2 + close + 2;
                continue;
            }
        }
        if let Some(close) = rest.strip_prefix('`').and_then(|r| r.find('`')) {
            if close > 0 {
                flush(&mut plain, &mut spans);
                spans.push(Span::Code(rest[1..1 + close].to_string()));
                i += 1 + close + 1;
                continue;
            }
        }
        if let Some(close) = rest.strip_prefix('*').and_then(|r| r.find('*')) {
            if close > 0 {
                flush(&mut plain, &mut spans);
                spans.push(Span::Italic(rest[1..1 + close].to_string()));
                i += 1 + close + 1;
                continue;
            }
        }
        if let Some(close) = rest.strip_prefix('_').and_then(|r| r.find('_')) {
            if close > 0 {
                flush(&mut plain, &mut spans);
                spans.push(Span::Italic(rest[1..1 + close].to_string()));
                i += 1 + close + 1;
                continue;
            }
        }

        let ch = line[i..].chars().next().unwrap();
        plain.push(ch);
        i += ch.len_utf8();
    }
    flush(&mut plain, &mut spans);
    spans
}

fn render_markdown(
    ui: &mut egui::Ui,
    content: &str,
    mentions: &HashMap<String, String>,
    channels: &HashMap<String, String>,
) {
    let mut rest = content;
    while !rest.is_empty() {
        if let Some(idx) = rest.find("```") {
            render_inline_block(ui, &rest[..idx], mentions, channels);
            let after = &rest[idx + 3..];
            match after.find("```") {
                Some(end) => {
                    let block = &after[..end];
                    let code = match block.find('\n') {
                        Some(nl) if !block[..nl].contains(' ') => &block[nl + 1..],
                        _ => block,
                    };
                    code_block(ui, code);
                    rest = &after[end + 3..];
                }
                None => {
                    code_block(ui, after);
                    rest = "";
                }
            }
        } else {
            render_inline_block(ui, rest, mentions, channels);
            rest = "";
        }
    }
}

fn render_inline_block(
    ui: &mut egui::Ui,
    text: &str,
    mentions: &HashMap<String, String>,
    channels: &HashMap<String, String>,
) {
    if text.is_empty() {
        return;
    }
    for line in text.split('\n') {
        if line.trim().is_empty() {
            ui.add_space(3.0);
            continue;
        }
        let spans = parse_inline(line, mentions, channels);
        ui.horizontal_wrapped(|ui| {
            for span in spans {
                match span {
                    Span::Text(s) => {
                        ui.label(egui::RichText::new(s).size(14.0).color(TEXT_MAIN));
                    }
                    Span::Bold(s) => {
                        ui.label(egui::RichText::new(s).size(14.0).strong().color(TEXT_MAIN));
                    }
                    Span::Italic(s) => {
                        ui.label(
                            egui::RichText::new(s).size(14.0).italics().color(TEXT_MAIN),
                        );
                    }
                    Span::Underline(s) => {
                        ui.label(
                            egui::RichText::new(s).size(14.0).underline().color(TEXT_MAIN),
                        );
                    }
                    Span::Strike(s) => {
                        ui.label(
                            egui::RichText::new(s)
                                .size(14.0)
                                .strikethrough()
                                .color(TEXT_DIM),
                        );
                    }
                    Span::Code(s) => {
                        ui.code(s);
                    }
                    Span::Mention(s) => {
                        ui.label(egui::RichText::new(s).size(14.0).strong().color(MENTION));
                    }
                    Span::ChannelLink(s) => {
                        ui.label(egui::RichText::new(s).size(14.0).strong().color(MENTION));
                    }
                    Span::Url(s) => {
                        ui.label(egui::RichText::new(s).size(14.0).underline().color(LINK));
                    }
                    Span::Emoji(s) => {
                        ui.label(egui::RichText::new(s).size(13.0).color(TEXT_DIM));
                    }
                }
            }
        });
    }
}

fn code_block(ui: &mut egui::Ui, code: &str) {
    egui::Frame::default()
        .fill(CODE_BG)
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.add(
                egui::Label::new(
                    egui::RichText::new(code.trim_end()).size(12.5).color(TEXT_MAIN),
                )
                .wrap(),
            );
        });
    ui.add_space(4.0);
}

// ------------------------------------------------------------ helpers

fn hash_color(seed: &str) -> Color32 {
    let idx = seed.bytes().map(|b| b as usize).sum::<usize>() % AVATAR_PALETTE.len();
    AVATAR_PALETTE[idx]
}

fn initial_of(name: &str) -> String {
    name.chars()
        .find(|c| c.is_alphanumeric())
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_else(|| "?".into())
}

fn fmt_time(ts: &str) -> Option<String> {
    DateTime::parse_from_rfc3339(ts)
        .ok()
        .map(|dt| dt.with_timezone(&Local).format("%H:%M").to_string())
}
