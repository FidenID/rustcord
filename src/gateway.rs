//! Klien Gateway WebSocket Discord: identify, heartbeat, dan event masuk.

use crate::models::Message;
use crate::state::UiMsg;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::sync::mpsc::Sender;
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::connect_async;

const GATEWAY_URL: &str = "wss://gateway.discord.gg/?v=10&encoding=json";

/// Perintah dari UI ke main gateway (mis. join voice → op 4).
#[derive(Debug)]
pub enum GatewayCmd {
    VoiceJoin { guild_id: String, channel_id: String },
    VoiceLeave,
    /// Update status diri (mute/deafen) di channel voice saat ini.
    VoiceSetFlags { mute: bool, deaf: bool },
}

/// Loop utama gateway: selalu mencoba menyambung ulang jika terputus.
pub async fn run(
    token: String,
    tx: Sender<UiMsg>,
    repaint: eframe::egui::Context,
    mut cmds: tokio::sync::mpsc::UnboundedReceiver<GatewayCmd>,
) {
    let (gen_tx, gen_rx) = tokio::sync::watch::channel(0u64);
    loop {
        match session(&token, &tx, &mut cmds, &gen_tx, gen_rx.clone(), &repaint).await {
            Ok(()) => {
                // Tidak seharusnya terjadi; session hanya kembali Err.
                return;
            }
            Err(e) => {
                let _ = tx.send(UiMsg::Disconnected(e.clone()));
                repaint.request_repaint();
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

/// Pasangan data yang menunggu disatukan untuk memulai voice.
#[derive(Default)]
struct VoicePair {
    /// (guild_id, session_id, channel_id) dari VOICE_STATE_UPDATE diri sendiri.
    state: Option<(String, String, String)>,
    /// (guild_id, token voice, endpoint) dari VOICE_SERVER_UPDATE.
    server: Option<(String, String, String)>,
}

/// Jika state + server sudah lengkap dan cocok, mulai task voice.
fn maybe_spawn_voice(
    voice: &mut VoicePair,
    me_id: &str,
    gen: &mut u64,
    gen_tx: &tokio::sync::watch::Sender<u64>,
    gen_rx: tokio::sync::watch::Receiver<u64>,
    tx: &Sender<UiMsg>,
    ctx: &eframe::egui::Context,
) {
    let (Some((g, s, c)), Some((g2, tok, ep))) =
        (voice.state.as_ref(), voice.server.as_ref())
    else {
        eprintln!(
            "[gw] pairing belum lengkap: state={} server={}",
            voice.state.is_some(),
            voice.server.is_some()
        );
        return;
    };
    if g != g2 {
        return;
    }
    let info = crate::voice::VoiceInfo {
        guild_id: g.clone(),
        channel_id: c.clone(),
        session_id: s.clone(),
        voice_token: tok.clone(),
        endpoint: ep.clone(),
        me_id: me_id.to_string(),
    };
    *gen += 1;
    let _ = gen_tx.send(*gen);
    voice.server = None; // tunggu server update berikutnya untuk join berikutnya
    tokio::spawn(crate::voice::start(
        info,
        gen_rx.clone(),
        *gen,
        tx.clone(),
        ctx.clone(),
    ));
}

/// Satu sesi gateway sampai terjadi error/koneksi tertutup.
async fn session(
    token: &str,
    tx: &Sender<UiMsg>,
    cmds: &mut tokio::sync::mpsc::UnboundedReceiver<GatewayCmd>,
    gen_tx: &tokio::sync::watch::Sender<u64>,
    gen_rx: tokio::sync::watch::Receiver<u64>,
    ctx: &eframe::egui::Context,
) -> Result<(), String> {
    let (ws, _) =
        connect_async(GATEWAY_URL).await.map_err(|e| {
            eprintln!("[gw] connect GAGAL: {e}");
            format!("gagal connect gateway: {e}")
        })?;
    eprintln!("[gw] terhubung, menunggu hello...");
    let (mut write, mut read) = ws.split();

    // 1. Tunggu HELLO (op 10) untuk mendapat heartbeat_interval.
    let hb_interval: u64;
    loop {
        let msg = read
            .next()
            .await
            .ok_or("gateway tertutup sebelum hello")?
            .map_err(|e| format!("gateway: {e}"))?;
        if let Some(v) = json_of(msg) {
            if v["op"].as_i64() == Some(10) {
                hb_interval = v["d"]["heartbeat_interval"].as_u64().unwrap_or(41_250);
                break;
            }
        }
    }

    // 2. IDENTIFY dengan user token.
    // Catatan: untuk akun user, field intents tidak wajib (itu aturan untuk bot).
    let identify = json!({
        "op": 2,
        "d": {
            "token": token,
            "properties": {
                "os": "linux",
                "browser": "Chrome",
                "device": "rustcord"
            }
        }
    });
    write
        .send(WsMessage::Text(identify.to_string().into()))
        .await
        .map_err(|e| format!("gagal kirim identify: {e}"))?;
    eprintln!("[gw] identify terkirim, heartbeat={hb_interval}ms");

    // 3. Loop baca event + kirim heartbeat sesuai jadwal.
    let mut seq: u64 = 0;
    let mut ticker = tokio::time::interval(Duration::from_millis(hb_interval));
    ticker.tick().await; // tick pertama langsung; abaikan
    let mut me_id = String::new();
    let mut voice = VoicePair::default();
    let mut gen: u64 = *gen_tx.borrow();
    // Channel voice tempat kita bergabung (untuk op 4 update flags).
    let mut voice_channel: Option<(String, String)> = None;

    loop {
        let item = tokio::select! {
            _ = ticker.tick() => {
                let hb = json!({ "op": 1, "d": seq });
                if write.send(WsMessage::Text(hb.to_string().into())).await.is_err() {
                    return Err("gagal kirim heartbeat".into());
                }
                continue;
            }
            cmd = cmds.recv() => match cmd {
                Some(GatewayCmd::VoiceJoin { guild_id, channel_id }) => {
                    gen += 1;
                    let _ = gen_tx.send(gen);
                    voice = VoicePair::default();
                    voice_channel = Some((guild_id.clone(), channel_id.clone()));
                    let op4 = json!({
                        "op": 4,
                        "d": {
                            "guild_id": guild_id,
                            "channel_id": channel_id,
                            "self_mute": true,
                            "self_deaf": false,
                        }
                    });
                    if write.send(WsMessage::Text(op4.to_string().into())).await.is_err() {
                        return Err("gagal kirim op4 voice".into());
                    }
                    eprintln!("[gw] op4 voice join terkirim");
                    continue;
                }
                Some(GatewayCmd::VoiceLeave) => {
                    gen += 1;
                    let _ = gen_tx.send(gen);
                    voice = VoicePair::default();
                    voice_channel = None;
                    let op4 = json!({ "op": 4, "d": { "guild_id": null, "channel_id": null, "self_mute": true, "self_deaf": true } });
                    let _ = write.send(WsMessage::Text(op4.to_string().into())).await;
                    eprintln!("[gw] op4 voice leave terkirim");
                    continue;
                }
                Some(GatewayCmd::VoiceSetFlags { mute, deaf }) => {
                    // Update flags tanpa pindah channel: ulangi guild/channel terakhir.
                    if let Some((g, c)) = voice_channel.clone() {
                        let op4 = json!({
                            "op": 4,
                            "d": {
                                "guild_id": g,
                                "channel_id": c,
                                "self_mute": mute,
                                "self_deaf": deaf,
                            }
                        });
                        let _ = write.send(WsMessage::Text(op4.to_string().into())).await;
                        eprintln!("[gw] op4 voice flags mute={mute} deaf={deaf}");
                    }
                    continue;
                }
                None => continue,
            },
            item = read.next() => match item {
                Some(i) => i,
                None => return Err("gateway tertutup".into()),
            },
        };

        let msg = item.map_err(|e| format!("gateway read: {e}"))?;
        if msg.is_ping() || msg.is_pong() {
            continue;
        }
        let Some(v) = json_of(msg) else { continue };

        if let Some(s) = v["s"].as_u64() {
            seq = s;
        }

        match v["op"].as_i64() {
            Some(0) => match v["t"].as_str().unwrap_or("") {
                "READY" => {
                    eprintln!("[gw] READY diterima");
                    me_id = v["d"]["user"]["id"].as_str().unwrap_or("").to_string();
                    let presences: std::collections::HashMap<String, String> = v["d"]
                        ["presences"]
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|p| {
                                    let id = p["user"]["id"].as_str()?.to_string();
                                    let st = p["status"].as_str()?.to_string();
                                    Some((id, st))
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    if !presences.is_empty() {
                        eprintln!("[gw] presence awal: {} user", presences.len());
                        let _ = tx.send(UiMsg::Presences(presences));
                    }
                    // Posisi voice awal semua orang (dari guild[].voice_states).
                    let mut voice_seed: std::collections::HashMap<
                        String,
                        std::collections::HashMap<String, Option<String>>,
                    > = std::collections::HashMap::new();
                    for g in v["d"]["guilds"].as_array().unwrap_or(&vec![]) {
                        let gid = match g["id"].as_str() {
                            Some(s) => s.to_string(),
                            None => continue,
                        };
                        for s in g["voice_states"].as_array().unwrap_or(&vec![]) {
                            let uid = s["user_id"]
                                .as_str()
                                .or_else(|| s["user"]["id"].as_str())
                                .unwrap_or("")
                                .to_string();
                            if uid.is_empty() {
                                continue;
                            }
                            let cid = s["channel_id"].as_str().map(|x| x.to_string());
                            voice_seed.entry(gid.clone()).or_default().insert(uid, cid);
                        }
                    }
                    let _ = tx.send(UiMsg::VoiceSeed(voice_seed));
                    let _ = tx.send(UiMsg::Status("Terhubung ke gateway".into()));
                }
                "PRESENCE_UPDATE" => {
                    let user_id =
                        v["d"]["user"]["id"].as_str().unwrap_or("").to_string();
                    let status =
                        v["d"]["status"].as_str().unwrap_or("offline").to_string();
                    if !user_id.is_empty() {
                        let _ = tx.send(UiMsg::PresenceUpdate { user_id, status });
                    }
                }
                "MESSAGE_CREATE" => {
                    if let Ok(m) = serde_json::from_value::<Message>(v["d"].clone()) {
                        eprintln!("[gw] pesan live di channel {}", m.channel_id);
                        let _ = tx.send(UiMsg::MessageCreate(m));
                    }
                }
                "TYPING_START" => {
                    let channel_id =
                        v["d"]["channel_id"].as_str().unwrap_or("").to_string();
                    let user_id = v["d"]["user_id"].as_str().unwrap_or("").to_string();
                    if !channel_id.is_empty() && !user_id.is_empty() {
                        let _ = tx.send(UiMsg::Typing { channel_id, user_id });
                    }
                }
                "VOICE_STATE_UPDATE" => {
                    let guild_id = v["d"]["guild_id"].as_str().unwrap_or("").to_string();
                    // d.user bisa kosong tergantung bentuk event; coba beberapa lokasi.
                    let user_id = v["d"]["user"]["id"]
                        .as_str()
                        .or_else(|| v["d"]["user_id"].as_str())
                        .or_else(|| v["d"]["member"]["user"]["id"].as_str())
                        .unwrap_or("")
                        .to_string();
                    let channel_id = v["d"]["channel_id"].as_str().map(|s| s.to_string());
                    let session_id = v["d"]["session_id"].as_str().unwrap_or("").to_string();
                    eprintln!(
                        "[gw] VOICE_STATE_UPDATE user={user_id} ch={channel_id:?} sess={}",
                        session_id.is_empty().then(|| "-").unwrap_or("ada")
                    );
                    if user_id.is_empty() {
                        continue; // tak bisa dipakai apa pun
                    }
                    let _ = tx.send(UiMsg::VoiceStateUpdate {
                        guild_id: guild_id.clone(),
                        channel_id: channel_id.clone(),
                        user_id: user_id.clone(),
                    });
                    // Data diri sendiri -> simpan untuk pairing voice.
                    if !me_id.is_empty() && user_id == me_id {
                        if let (Some(ch), false) = (&channel_id, session_id.is_empty()) {
                            voice.state = Some((guild_id, session_id, ch.clone()));
                        } else {
                            voice.state = None; // keluar voice
                        }
                        maybe_spawn_voice(&mut voice, &me_id, &mut gen, gen_tx, gen_rx.clone(), tx, ctx);
                    }
                }
                "VOICE_SERVER_UPDATE" => {
                    let guild_id = v["d"]["guild_id"].as_str().unwrap_or("").to_string();
                    let voice_token = v["d"]["token"].as_str().unwrap_or("").to_string();
                    let endpoint = v["d"]["endpoint"].as_str().unwrap_or("").to_string();
                    eprintln!("[gw] VOICE_SERVER_UPDATE guild={guild_id} ep={endpoint}");
                    voice.server = Some((guild_id, voice_token, endpoint));
                    maybe_spawn_voice(&mut voice, &me_id, &mut gen, gen_tx, gen_rx.clone(), tx, ctx);
                }
                other => {
                    eprintln!("[gw] event: {other}");
                }
            },
            Some(11) => {}               // heartbeat ACK
            Some(9) => return Err("invalid session (op 9)".into()),
            Some(7) => return Err("server minta reconnect (op 7)".into()),
            Some(1) => {
                // Server minta heartbeat segera
                let hb = json!({ "op": 1, "d": seq });
                let _ = write.send(WsMessage::Text(hb.to_string().into())).await;
            }
            other => {
                eprintln!("[gw] op: {other:?}");
            }
        }
    }
}

fn json_of(msg: WsMessage) -> Option<Value> {
    match msg {
        WsMessage::Text(t) => serde_json::from_str(&t).ok(),
        WsMessage::Binary(b) => serde_json::from_slice(&b).ok(),
        _ => None,
    }
}
