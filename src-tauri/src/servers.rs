//! A small Minecraft Java status protocol client and a persistent local server library.
//! Minecraft server status has no authentication; connection attempts are user-initiated.
use serde::Serialize;
use serde_json::Value;
use std::{net::IpAddr, time::{Duration, Instant}};
use rusqlite::params;
use tauri::AppHandle;
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream, time::timeout};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all="camelCase")]
pub struct SavedServer {
    pub id: String,
    pub name: String,
    pub address: String,
    pub port: u16,
    pub favorite: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all="camelCase")]
pub struct ServerStatus {
    pub online: bool,
    pub version: Option<String>,
    pub players_online: Option<u32>,
    pub max_players: Option<u32>,
    pub description: Option<String>,
    pub latency_ms: Option<u128>,
    pub error: Option<String>,
}

fn database(app: &AppHandle) -> Result<rusqlite::Connection, String> {
    let db = super::instances::db(app)?;
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS saved_servers (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            address TEXT NOT NULL,
            port INTEGER NOT NULL CHECK(port BETWEEN 1 AND 65535),
            favorite INTEGER NOT NULL DEFAULT 0
        );"
    ).map_err(|e| e.to_string())?;
    Ok(db)
}

fn validate_server(name: &str, address: &str, port: u16) -> Result<(), String> {
    if name.is_empty() || name.chars().count() > 64 || name.chars().any(char::is_control) {
        return Err("Назва сервера має містити 1–64 символи без керівних символів".into());
    }
    if port == 0 { return Err("Порт має бути від 1 до 65535".into()); }
    if address.is_empty() || address.len() > 253 || address.chars().any(char::is_control) {
        return Err("Некоректна адреса сервера".into());
    }
    // Permit IP literals (including IPv6) and ordinary DNS hostnames, not URLs/paths.
    if address.parse::<IpAddr>().is_ok() { return Ok(()); }
    if !address.is_ascii() || address.starts_with('.') || address.ends_with('.')
        || address.split('.').any(|label| label.is_empty() || label.len() > 63
             || label.starts_with('-') || label.ends_with('-')
             || !label.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'))
    {
        return Err("Вкажи DNS-ім'я або IP без http:// і без порту".into());
    }
    Ok(())
}

fn row_server(row: &rusqlite::Row<'_>) -> rusqlite::Result<SavedServer> {
    Ok(SavedServer {
        id: row.get(0)?,
        name: row.get(1)?,
        address: row.get(2)?,
        port: row.get(3)?,
        favorite: row.get::<_, i64>(4)? != 0,
    })
}

#[tauri::command]
pub fn list_servers(app: AppHandle) -> Result<Vec<SavedServer>, String> {
    let db = database(&app)?;
    let mut stmt = db.prepare(
        "SELECT id,name,address,port,favorite FROM saved_servers ORDER BY favorite DESC, name COLLATE NOCASE"
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], row_server).map_err(|e| e.to_string())?;
    let result: Result<Vec<SavedServer>, String> = rows.map(|r| r.map_err(|e| e.to_string())).collect();
    result
}

#[tauri::command]
pub fn save_server(app: AppHandle, id: Option<String>, name: String, address: String, port: u16, favorite: bool) -> Result<SavedServer, String> {
    let name = name.trim().to_owned();
    let address = address.trim().to_owned();
    validate_server(&name, &address, port)?;
    let db = database(&app)?;
    let id = if let Some(id) = id {
        Uuid::parse_str(&id).map_err(|_| "Invalid server id")?;
        if db.execute(
            "UPDATE saved_servers SET name=?2,address=?3,port=?4,favorite=?5 WHERE id=?1",
            params![id,name,address,port,i64::from(favorite)]
        ).map_err(|e| e.to_string())? == 0 {
            return Err("Сервер не знайдено".into());
        }
        id
    } else {
        let id = Uuid::new_v4().to_string();
        db.execute(
            "INSERT INTO saved_servers(id,name,address,port,favorite) VALUES(?1,?2,?3,?4,?5)",
            params![id,name,address,port,i64::from(favorite)]
        ).map_err(|e| e.to_string())?;
        id
    };
    Ok(SavedServer {id,name,address,port,favorite})
}

#[tauri::command]
pub fn delete_server(app: AppHandle, id: String) -> Result<(), String> {
    Uuid::parse_str(&id).map_err(|_| "Invalid server id")?;
    let db = database(&app)?;
    if db.execute("DELETE FROM saved_servers WHERE id=?1",params![id])
        .map_err(|e| e.to_string())? == 0 {
        return Err("Сервер не знайдено".into());
    }
    Ok(())
}

fn encode_varint(mut number: u32, dest: &mut Vec<u8>) {
    loop {
        if number & !0x7f == 0 {dest.push(number as u8);return;}
        dest.push((number as u8 & 0x7f) | 0x80);
        number >>= 7;
    }
}
fn encode_string(text: &str, out: &mut Vec<u8>) -> Result<(), String> {
    if text.len() > 253 {return Err("Address too long".into());}
    encode_varint(text.len() as u32,out);
    out.extend_from_slice(text.as_bytes());
    Ok(())
}
async fn read_varint(stream: &mut TcpStream) -> Result<u32, String> {
    let mut result = 0u32;
    for shift in [0,7,14,21,28] {
        let value = stream.read_u8().await.map_err(|e| e.to_string())?;
        result |= ((value & 0x7f) as u32) << shift;
        if value & 0x80 == 0 {return Ok(result);}
    }
    Err("Invalid Minecraft response length".into())
}

fn text_component(value: &Value, depth: usize) -> String {
    if depth >= 6 {return String::new();}
    match value {
        Value::String(s) => s.clone(),
        Value::Object(map) => {
            let mut out = map.get("text").and_then(Value::as_str).unwrap_or("").to_owned();
            if let Some(extra) = map.get("extra").and_then(Value::as_array) {
                for component in extra.iter().take(30) {
                    if out.len() >= 450 {break;}
                    out.push_str(&text_component(component,depth+1));
                }
            }
            out
        },
        _ => String::new()
    }
}

async fn status_request(address: &str, port: u16) -> Result<ServerStatus, String> {
    let start = Instant::now();
    let mut stream = TcpStream::connect((address,port)).await.map_err(|e| e.to_string())?;
    let mut handshake = vec![0u8]; // handshake packet ID
    encode_varint(767, &mut handshake); // status requests ignore the client protocol version
    encode_string(address, &mut handshake)?;
    handshake.extend_from_slice(&port.to_be_bytes());
    encode_varint(1,&mut handshake); // next state=status
    let mut packet = Vec::with_capacity(handshake.len()+5);
    encode_varint(handshake.len() as u32,&mut packet);
    packet.extend_from_slice(&handshake);
    stream.write_all(&packet).await.map_err(|e| e.to_string())?;
    stream.write_all(&[1,0]).await.map_err(|e| e.to_string())?; // status request
    let size = read_varint(&mut stream).await?;
    if size == 0 || size > 1_048_576 {
        return Err("Minecraft status response exceeds safety limit".into());
    }
    let pid = read_varint(&mut stream).await?;
    if pid != 0 {return Err("Unexpected Minecraft status packet".into());}
    let string_size = read_varint(&mut stream).await?;
    if string_size == 0 || string_size > 1_000_000 || string_size > size {
        return Err("Invalid Minecraft status JSON length".into());
    }
    let mut bytes = vec![0u8;string_size as usize];
    stream.read_exact(&mut bytes).await.map_err(|e| e.to_string())?;
    let doc: Value = serde_json::from_slice(&bytes).map_err(|e| format!("Invalid Minecraft status JSON: {e}"))?;
    let description = doc.get("description").map(|v|text_component(v,0));
    Ok(ServerStatus {
        online:true,
        version: doc["version"]["name"].as_str().map(ToOwned::to_owned),
        players_online: doc["players"]["online"].as_u64().and_then(|x|u32::try_from(x).ok()),
        max_players: doc["players"]["max"].as_u64().and_then(|x|u32::try_from(x).ok()),
        description:description.map(|s|s.chars().take(450).collect()),
        latency_ms:Some(start.elapsed().as_millis()),
        error:None
    })
}

#[tauri::command]
pub async fn check_server_status(address:String, port:u16) -> Result<ServerStatus, String> {
    validate_server("Minecraft Server",address.trim(),port)?;
    let result = timeout(Duration::from_secs(8),status_request(address.trim(),port)).await;
    Ok(match result {
        Ok(Ok(status))=>status,
        Ok(Err(error))=>ServerStatus {
            online:false,version:None,players_online:None,max_players:None,
            description:None,latency_ms:None,error:Some(error.chars().take(180).collect())
        },
        Err(_)=>ServerStatus {
            online:false,version:None,players_online:None,max_players:None,
            description:None,latency_ms:None,error:Some("Час очікування відповіді минув (8 секунд)".into())
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validation_prevents_urls() {
        assert!(validate_server("Home","mc.example.org",25565).is_ok());
        assert!(validate_server("Home","127.0.0.1",25565).is_ok());
        assert!(validate_server("Home","::1",25565).is_ok());
        assert!(validate_server("Home","https://example.org",25565).is_err());
        assert!(validate_server("Home","../../etc/passwd",25565).is_err());
        assert!(validate_server("Home","mc.example.org",0).is_err());
    }
    #[test]
    fn varint_encoding() {
        let mut bytes=Vec::new();
        encode_varint(300,&mut bytes);
        assert_eq!(bytes,vec![0xac,0x02]);
    }
    #[test]
    fn status_client_parses_mocked_java_server() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all().build().expect("tokio runtime");
        runtime.block_on(async {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind test server");
            let port = listener.local_addr().expect("local address").port();
            let fake = tokio::spawn(async move {
                let (mut socket, _) = listener.accept().await.expect("accept client");
                let length = read_varint(&mut socket).await.expect("request length");
                let mut handshake = vec![0u8; length as usize];
                socket.read_exact(&mut handshake).await.expect("handshake");
                assert_eq!(read_varint(&mut socket).await.expect("status length"),1);
                assert_eq!(read_varint(&mut socket).await.expect("status request packet id"),0);
                let json = serde_json::json!({
                    "version":{"name":"1.21.4"},
                    "players":{"online":3,"max":30},
                    "description":{"text":"Привіт", "extra":[{"text":" світ"}]}
                }).to_string();
                let mut payload = vec![0u8];
                encode_varint(json.len() as u32,&mut payload);
                payload.extend_from_slice(json.as_bytes());
                let mut response=Vec::new();
                encode_varint(payload.len() as u32,&mut response);
                response.extend_from_slice(&payload);
                socket.write_all(&response).await.expect("send response");
            });
            let state=status_request("127.0.0.1",port).await.expect("read Java status");
            assert!(state.online);
            assert_eq!(state.version.as_deref(),Some("1.21.4"));
            assert_eq!(state.players_online,Some(3));
            assert_eq!(state.max_players,Some(30));
            assert_eq!(state.description.as_deref(),Some("Привіт світ"));
            fake.await.expect("mock server");
        });
    }
    #[test]
    fn status_rejects_oversized_packets() {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("tokio");
        runtime.block_on(async {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
            let port=listener.local_addr().expect("port").port();
            let fake=tokio::spawn(async move {
                let (mut stream,_) = listener.accept().await.expect("connection");
                let length=read_varint(&mut stream).await.expect("handshake length");
                let mut data=vec![0;length as usize];
                stream.read_exact(&mut data).await.expect("handshake data");
                stream.read_exact(&mut [0u8;2]).await.expect("status request");
                let mut invalid=Vec::new();
                encode_varint(1_048_577,&mut invalid);
                stream.write_all(&invalid).await.expect("send invalid length");
            });
            let result=status_request("127.0.0.1",port).await;
            assert!(result.is_err());
            fake.await.expect("fake server");
        });
    }
    #[test]
    fn description_components() {
        let value:Value=serde_json::json!({"text":"Привіт", "extra":[{"text":" світ"}]});
        assert_eq!(text_component(&value,0),"Привіт світ");
    }
}
