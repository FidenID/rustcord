# Rustcord

Klien Discord native yang ringan, ditulis dengan Rust (egui + wgpu).
Dibuat untuk pemakaian pribadi, login dengan token.

## Fitur (MVP)
- Login dengan token Discord (tersimpan di `~/.config/rustcord/token.txt`)
- Daftar server & channel (termasuk kategori, bisa dilipat)
- Daftar DM, pesan real-time via Gateway WebSocket
- Kirim pesan, indikator belum dibaca
- UI 3 kolom ala Discord, tema gelap

## Build & jalankan
```sh
cargo run --release
```

## Peringatan
Klien unofficial melanggar Terms of Service Discord dan berisiko banned akun.
Gunakan dengan risiko sendiri. Rustcord tidak berafiliasi dengan Discord Inc.
