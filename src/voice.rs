//! Voice Phase V1 (dengar saja):
//! voice gateway WebSocket -> UDP discovery -> dekripsi RTP (XSalsa20Poly1305)
//! -> dekode Opus -> playback via rodio.
//!
//! Alur (sama seperti semua klien unofficial, ditulis ulang sendiri):
//! 1. Main gateway mengirim op 4 -> Discord balas VOICE_SERVER_UPDATE + VOICE_STATE_UPDATE.
//! 2. Koneksi voice gateway `wss://<endpoint>?v=4`, identify, terima op 2 Ready.
//! 3. UDP discovery ke ip:port dari Ready -> ketahuan IP/publik kita.
//! 4. Select Protocol mode `xsalsa20_poly1305` -> op 4 Session Description (secret key).
//! 5. Paket RTP diterima, didekripsi, didekode Opus 48 kHz stereo, diputar.

use crate::state::UiMsg;
use crypto_secretbox::aead::{AeadInPlace, KeyInit};
use crypto_secretbox::aead::generic_array::GenericArray;
use crypto_secretbox::XSalsa20Poly1305;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::sync::watch;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message as WsMessage;

/// Data pairing dari main gateway (VOICE_STATE_UPDATE + VOICE_SERVER_UPDATE).
pub struct VoiceInfo {
    pub guild_id: String,
    pub channel_id: String,
    pub session_id: String,
    pub voice_token: String,
    pub endpoint: String,
    pub me_id: String,
}

/// Jalankan satu sesi suara. Keluar sendiri jika `gen` berubah (join baru).
pub async fn start(
    info: VoiceInfo,
    mut gen_rx: watch::Receiver<u64>,
    gen: u64,
    ui: Sender<UiMsg>,
    ctx: eframe::egui::Context,
) {
    let _ = gen;
    // Discord menolak request tanpa path (`GET ?v=4` = 400), jadi pakai `/?v=4`.
    let url = format!("wss://{}/?v=4", info.endpoint.trim_end_matches('/'));
    let (ws, _) = match connect_async(&url).await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[voice] connect GAGAL: {e}");
            let _ = ui.send(UiMsg::VoiceFailed(format!("Gagal sambung voice: {e}")));
            return;
        }
    };
    eprintln!("[voice] voice gateway terhubung");
    let (mut write, mut read) = ws.split();

    // Tunggu op 8 Hello untuk interval heartbeat.
    let hb_interval = loop {
        let Some(v) = next_json(&mut read).await else {
            let _ = ui.send(UiMsg::VoiceFailed("Voice gateway tertutup".into()));
            return;
        };
        if v["op"].as_i64() == Some(8) {
            break v["d"]["heartbeat_interval"].as_u64().unwrap_or(13_750);
        }
    };

    // Identify: token voice (BUKAN token user), session id, server id.
    let identify = json!({
        "op": 0,
        "d": {
            "token": info.voice_token,
            "user_id": info.me_id,
            "server_id": info.guild_id,
            "session_id": info.session_id,
        }
    });
    if write.send(WsMessage::Text(identify.to_string().into())).await.is_err() {
        let _ = ui.send(UiMsg::VoiceFailed("Gagal kirim identify voice".into()));
        return;
    }
    eprintln!("[voice] identify terkirim");

    // Ready (op 2): ssrc, ip, port, modes.
    let (ssrc, server_ip, server_port, _modes) = loop {
        let Some(v) = next_json(&mut read).await else {
            let _ = ui.send(UiMsg::VoiceFailed("Voice gateway tertutup".into()));
            return;
        };
        if v["op"].as_i64() == Some(2) {
            let ssrc = v["d"]["ssrc"].as_u64().unwrap_or(0) as u32;
            let ip = v["d"]["ip"].as_str().unwrap_or("").to_string();
            let port = v["d"]["port"].as_u64().unwrap_or(0) as u16;
            break (ssrc, ip, port, ());
        }
    };
    eprintln!("[voice] ready: ssrc={ssrc} server={server_ip}:{server_port}");

    // UDP discovery: kirim 70 byte, server balas IP publik + port kita.
    let udp = match UdpSocket::bind("0.0.0.0:0").await {
        Ok(u) => u,
        Err(e) => {
            let _ = ui.send(UiMsg::VoiceFailed(format!("Gagal buka UDP: {e}")));
            return;
        }
    };
    if udp.connect((server_ip.as_str(), server_port)).await.is_err() {
        let _ = ui.send(UiMsg::VoiceFailed("Gagal koneksi UDP".into()));
        return;
    }
    let mut disc = Vec::with_capacity(74);
    disc.extend_from_slice(&[0x1, 0x00, 0x46]); // opcode 1, panjang 70
    disc.extend_from_slice(&ssrc.to_be_bytes());
    disc.extend(std::iter::repeat_n(0u8, 64));
    if udp.send(&disc).await.is_err() {
        let _ = ui.send(UiMsg::VoiceFailed("Gagal kirim UDP discovery".into()));
        return;
    }
    let mut resp = [0u8; 74];
    if udp.recv(&mut resp).await.is_err() {
        let _ = ui.send(UiMsg::VoiceFailed("Tidak ada balasan UDP discovery".into()));
        return;
    }
    let my_ip = String::from_utf8_lossy(&resp[8..70])
        .trim_end_matches('\0')
        .to_string();
    let my_port = u16::from_be_bytes([resp[70], resp[71]]);
    eprintln!("[voice] discovery: {my_ip}:{my_port}");

    // Select Protocol.
    let select = json!({
        "op": 1,
        "d": {
            "protocol": "udp",
            "data": {
                "address": my_ip,
                "port": my_port,
                "mode": "xsalsa20_poly1305",
            }
        }
    });
    if write.send(WsMessage::Text(select.to_string().into())).await.is_err() {
        let _ = ui.send(UiMsg::VoiceFailed("Gagal select protocol".into()));
        return;
    }

    // Session Description (op 4): secret key 32 byte.
    let secret_key: Vec<u8> = loop {
        let Some(v) = next_json(&mut read).await else {
            let _ = ui.send(UiMsg::VoiceFailed("Voice gateway tertutup".into()));
            return;
        };
        if v["op"].as_i64() == Some(4) {
            let mut key = Vec::new();
            for x in v["d"]["secret_key"].as_array().unwrap_or(&vec![]) {
                key.push(x.as_u64().unwrap_or(0) as u8);
            }
            break key;
        }
    };
    eprintln!("[voice] secret key didapat — mulai mendengarkan");

    // Cipher dibuat sekali; kalau key tidak 32 byte, batal.
    let Ok(cipher) = XSalsa20Poly1305::new_from_slice(&secret_key) else {
        let _ = ui.send(UiMsg::VoiceFailed("Secret key voice tidak valid".into()));
        return;
    };

    // Pemutar audio (thread std; sink butuh realtime).
    let queue: Arc<Mutex<VecDeque<f32>>> = Arc::new(Mutex::new(VecDeque::new()));
    let audio_stop = spawn_audio_player(queue.clone());

    // Loop penerima UDP: dekripsi + dekode + dorong ke queue audio.
    let mut decoders: HashMap<u32, opus::Decoder> = HashMap::new();
    let buf = vec![0u8; 2048];

    // Task UDP berjalan di samping loop WS (heartbeat).
    let udp_task = tokio::spawn({
        let mut gen_rx = gen_rx.clone();
        let mut buf = buf.clone();
        async move {
            loop {
                tokio::select! {
                    _ = gen_rx.changed() => return, // join baru -> matikan
                    r = udp.recv(&mut buf) => {
                        let n = match r { Ok(n) => n, Err(_) => return };
                        if n < 12 { continue; }
                        let pkt = &buf[..n];
                        if pkt[0] & 0x7F != 0x78 { continue; } // bukan frame suara
                        // Header RTP: 12 byte, +4 jika bit extension (Discord menambah 1 ekstensi).
                        let hlen = if pkt[0] & 0x10 != 0 { 16 } else { 12 };
                        if n <= hlen + 16 { continue; } // minimal opus + tag
                        let ssrc = u32::from_be_bytes([pkt[8], pkt[9], pkt[10], pkt[11]]);
                        // Nonce = header (hlen byte) di-pad nol ke 24 byte.
                        let mut nonce = [0u8; 24];
                        nonce[..hlen].copy_from_slice(&pkt[..hlen]);
                        // Ciphertext = payload; 16 byte terakhir adalah MAC (AEAD tag).
                        let mut data = pkt[hlen..].to_vec();
                        let tag = data.split_off(data.len() - 16);
                        if cipher
                            .decrypt_in_place_detached(
                                GenericArray::from_slice(&nonce),
                                &[],
                                &mut data,
                                GenericArray::from_slice(&tag),
                            )
                            .is_err()
                        {
                            continue; // paket rusak/drop
                        }
                        // Dekode Opus -> f32 stereo 48 kHz.
                        let dec = decoders
                            .entry(ssrc)
                            .or_insert_with(|| {
                                opus::Decoder::new(48000, opus::Channels::Stereo).ok().unwrap()
                            });
                        let mut out = [0i16; 960 * 2];
                        let samples = match dec.decode(&data, &mut out, false) {
                            Ok(s) => s,
                            Err(_) => continue,
                        };
                        let mut q = queue.lock().unwrap();
                        for i in 0..samples * 2 {
                            q.push_back(out[i] as f32 / 32768.0);
                        }
                        // Jaga buffer tidak menumpuk (max 200ms).
                        while q.len() > 48_000 / 5 {
                            q.pop_front();
                        }
                    }
                }
            }
        }
    });

    let _ = ui.send(UiMsg::VoiceConnected {
        guild_id: info.guild_id.clone(),
        channel_id: info.channel_id.clone(),
    });
    ctx.request_repaint();

    // Loop WS: heartbeat + deteksi penutupan / generasi baru.
    let mut ticker = tokio::time::interval(Duration::from_millis(hb_interval));
    ticker.tick().await;
    let mut nonce: u32 = 1234;
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                nonce = nonce.wrapping_add(1);
                let hb = json!({ "op": 3, "d": nonce });
                if write.send(WsMessage::Text(hb.to_string().into())).await.is_err() {
                    break;
                }
            }
            _ = gen_rx.changed() => {
                break; // join channel lain / leave
            }
            item = read.next() => {
                if item.is_none() { break; }
                // op 13 (client disconnect) dll diabaikan.
            }
        }
    }
    udp_task.abort();
    // Berhenti memutar: sumber audio mengembalikan None saat flag disetel.
    audio_stop.store(true, std::sync::atomic::Ordering::Relaxed);
    let _ = ui.send(UiMsg::VoiceLeft);
    ctx.request_repaint();
    eprintln!("[voice] sesi suara berakhir");
}

/// Thread playback: baca antrian f32 dan mainkan ke perangkat default.
/// Mengembalikan flag stop — disetel saat sesi suara berakhir agar sumber berhenti.
fn spawn_audio_player(queue: Arc<Mutex<VecDeque<f32>>>) -> Arc<AtomicBool> {
    let stop = Arc::new(AtomicBool::new(false));
    let stop2 = stop.clone();
    std::thread::spawn(move || {
        let stream = match rodio::OutputStreamBuilder::open_default_stream() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[voice] audio GAGAL: {e}");
                return;
            }
        };
        let sink = rodio::Sink::connect_new(stream.mixer());
        sink.append(VoiceSource { q: queue, stop: stop2 });
        sink.sleep_until_end();
    });
    stop
}

/// Sumber audio sederhana: pop antrian, atau hening jika kosong.
struct VoiceSource {
    q: Arc<Mutex<VecDeque<f32>>>,
    stop: Arc<AtomicBool>,
}

impl Iterator for VoiceSource {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if self.stop.load(Ordering::Relaxed) {
            return None; // sesi berakhir -> sink selesai
        }
        Some(self.q.lock().unwrap().pop_front().unwrap_or(0.0))
    }
}

impl rodio::Source for VoiceSource {
    fn current_span_len(&self) -> Option<usize> {
        None // span tak terbatas; iterator yang mengatur
    }
    fn channels(&self) -> u16 {
        2
    }
    fn sample_rate(&self) -> u32 {
        48_000
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

async fn next_json<S>(read: &mut S) -> Option<Value>
where
    S: futures_util::Stream<
            Item = Result<WsMessage, tokio_tungstenite::tungstenite::Error>,
        > + Unpin,
{
    loop {
        let msg = read.next().await?;
        match msg {
            Ok(WsMessage::Text(t)) => return serde_json::from_str(&t).ok(),
            Ok(_) => continue,
            Err(_) => return None,
        }
    }
}
