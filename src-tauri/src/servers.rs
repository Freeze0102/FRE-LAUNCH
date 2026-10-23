//! Minecraft Java Edition status polling and persistent server favorites.
use rusqlite::params;
use serde::Serialize;
use std::time::Instant;
use tauri::AppHandle;
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream, time::{timeout, Duration}};
use uuid::Uuid;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Server {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatus {
    pub online: bool,
    pub ping_ms: u128,
    pub version: String,
    pub players_online: u32,
    pub players_max: u32,
    pub description: String,
}

fn server_db(app: &AppHandle) -> Result<rusqlite::Connection, String> {
    let connection = super::instances::db(app)?;
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS saved_servers (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            host TEXT NOT NULL,
            port INTEGER NOT NULL
        );"
    ).map_err(|e| e.to_string())?;
    Ok(connection)
}
fn validate_host(host: &str) -> Result<(), String> {
    if host.is_empty() || host.len()>253
        || !host.bytes().all(|c|c.is_ascii_alphanumeric()||matches!(c,b'.'|b'-'|b'_'|b':'))
        || host.starts_with('-') || host.contains("..") {
        return Err("Invalid Minecraft server hostname".into());
    }
    Ok(())
}
fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.chars().count()>50 || name.chars().any(char::is_control) {
        return Err("Server name must be 1–50 characters".into());
    }
    Ok(())
}
fn row_server(row: &rusqlite::Row<'_>) -> rusqlite::Result<Server> {
    Ok(Server {
        id: row.get(0)?,
        name: row.get(1)?,
        host: row.get(2)?,
        port: row.get(3)?,
    })
}

#[tauri::command]
pub fn list_servers(app: AppHandle) -> Result<Vec<Server>, String> {
    let connection = server_db(&app)?;
    let mut stmt = connection.prepare("SELECT id,name,host,port FROM saved_servers ORDER BY name COLLATE NOCASE")
        .map_err(|e| e.to_string())?;
    let items = stmt.query_map([], row_server).map_err(|e| e.to_string())?;
    items.map(|row|row.map_err(|e|e.to_string())).collect()
}
#[tauri::command]
pub fn add_server(app: AppHandle, name: String, host: String, port: u16) -> Result<Server, String> {
    validate_name(name.trim())?;
    validate_host(host.trim())?;
    if port == 0 {return Err("Port must be between 1 and 65535".into());}
    let server = Server{id:Uuid::new_v4().to_string(),name:name.trim().to_string(),host:host.trim().to_string(),port};
    server_db(&app)?.execute("INSERT INTO saved_servers (id,name,host,port) VALUES (?1,?2,?3,?4)",
        params![server.id,server.name,server.host,server.port]).map_err(|e| e.to_string())?;
    Ok(server)
}
#[tauri::command]
pub fn delete_server(app: AppHandle, id: String) -> Result<(), String> {
    Uuid::parse_str(&id).map_err(|_|"Invalid server ID".to_string())?;
    server_db(&app)?.execute("DELETE FROM saved_servers WHERE id=?1",params![id])
        .map_err(|e|e.to_string())?;
    Ok(())
}

fn write_varint(mut value: u32, output: &mut Vec<u8>) {
    loop {
        if value & !0x7f == 0 { output.push(value as u8); break; }
        output.push(((value & 0x7f) as u8)|0x80);
        value >>= 7;
    }
}
async fn read_varint(stream: &mut TcpStream) -> Result<u32, String> {
    let mut result=0u32;
    for shift in (0..=28).step_by(7) {
        let mut data=[0u8;1];
        stream.read_exact(&mut data).await.map_err(|e|e.to_string())?;
        let byte=data[0];
        result |= ((byte & 0x7f) as u32)<<shift;
        if byte & 0x80 == 0 {return Ok(result);}
    }
    Err("Invalid Minecraft packet length".into())
}
fn write_string(text: &str, output: &mut Vec<u8>) {
    write_varint(text.len() as u32,output);
    output.extend_from_slice(text.as_bytes());
}
async fn send_packet(stream: &mut TcpStream, payload: &[u8]) -> Result<(), String> {
    let mut packet=Vec::with_capacity(payload.len()+5);
    write_varint(payload.len() as u32,&mut packet);
    packet.extend_from_slice(payload);
    stream.write_all(&packet).await.map_err(|e|e.to_string())
}
fn description_plain(value: &serde_json::Value) -> String {
    if let Some(raw)=value.as_str(){return raw.chars().take(120).collect();}
    if let Some(text)=value.get("text").and_then(serde_json::Value::as_str){
        return text.chars().take(120).collect();
    }
    String::new()
}
async fn poll(host:&str,port:u16)->Result<ServerStatus,String>{
    validate_host(host)?;
    let since=Instant::now();
    let address=format!("{host}:{port}");
    let mut socket=timeout(Duration::from_secs(6),TcpStream::connect(address))
        .await.map_err(|_|"Timed out connecting to Minecraft server".to_string())?
        .map_err(|e|format!("Cannot connect: {e}"))?;
    let mut handshake=vec![0u8];
    write_varint(760,&mut handshake);
    write_string(host,&mut handshake);
    handshake.extend_from_slice(&port.to_be_bytes());
    write_varint(1,&mut handshake);
    send_packet(&mut socket,&handshake).await?;
    send_packet(&mut socket,&[0]).await?;
    let length=read_varint(&mut socket).await?;
    if length<3 || length>32768 {return Err("Invalid Minecraft server status packet".into());}
    let mut payload=vec![0u8;length as usize];
    socket.read_exact(&mut payload).await.map_err(|e|e.to_string())?;
    if payload[0]!=0 {return Err("Unexpected Minecraft packet ID".into());}
    let mut index=1;
    let mut count=0u32;
    let mut shift=0;
    loop{
        if index>=payload.len() || shift>28 {return Err("Invalid Minecraft response length".into());}
        let byte=payload[index];
        index+=1;
        count|=((byte & 0x7f) as u32)<<shift;
        if byte & 0x80 == 0 {break;}
        shift+=7;
    }
    if count as usize>payload.len()-index {return Err("Truncated Minecraft status JSON".into());}
    let json:serde_json::Value=serde_json::from_slice(&payload[index..index+count as usize])
        .map_err(|e|format!("Invalid Minecraft server status: {e}"))?;
    Ok(ServerStatus{
        online:true,
        ping_ms:since.elapsed().as_millis(),
        version:json["version"]["name"].as_str().unwrap_or("Unknown").chars().take(80).collect(),
        players_online:json["players"]["online"].as_u64().unwrap_or(0).min(u32::MAX as u64) as u32,
        players_max:json["players"]["max"].as_u64().unwrap_or(0).min(u32::MAX as u64) as u32,
        description:description_plain(&json["description"]),
    })
}
#[tauri::command]
pub async fn ping_server(host: String, port: u16) -> Result<ServerStatus, String> {
    if port==0 {return Err("Invalid port".into());}
    timeout(Duration::from_secs(10),poll(&host,port))
        .await.map_err(|_|"Server ping timed out".to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hostname_checks(){
        assert!(validate_host("mc.example.org").is_ok());
        assert!(validate_host("127.0.0.1").is_ok());
        assert!(validate_host("evil/path").is_err());
        assert!(validate_host("a..b").is_err());
    }
    #[test]
    fn encode_varint(){
        let mut out=Vec::new();
        write_varint(300,&mut out);
        assert_eq!(out,vec![0xac,0x02]);
    }
}
