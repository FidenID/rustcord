// Tes cepat format URL voice gateway Discord.
use tokio_tungstenite::connect_async;

#[tokio::main]
async fn main() {
    let host = std::env::args().nth(1).expect("pemakaian: vws <endpoint[:port]>");
    let urls = [
        format!("wss://{host}?v=4"),
        format!("wss://{host}/?v=4"),
        format!("wss://{host}"),
        format!("wss://{host}/?v=5"),
        format!("wss://{host}/?v=4&encoding=json"),
        format!("wss://{host}/?v=4&encoding=json&v=4"),
    ];
    for u in &urls {
        match connect_async(u.as_str()).await {
            Ok((mut ws, resp)) => {
                println!("OK   {u}  status={:?}", resp.status());
                // baca 1 pesan pertama
                use futures_util::StreamExt;
                match tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    ws.next(),
                )
                .await
                {
                    Ok(Some(Ok(m))) => println!("     pesan pertama: {}", m),
                    Ok(Some(Err(e))) => println!("     err baca: {e}"),
                    _ => println!("     tidak ada pesan"),
                }
            }
            Err(e) => println!("GAGAL {u}  -> {e}"),
        }
    }
}
