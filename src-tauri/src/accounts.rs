use rusqlite::params;
use serde::Serialize;
use tauri::AppHandle;
use uuid::Uuid;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfflineAccount {
    pub id: String,
    pub nickname: String,
    pub offline_uuid: String,
}

pub fn offline_uuid(name: &str) -> String {
    let hash = md5::compute(format!("OfflinePlayer:{name}"));
    let mut bytes = hash.0;
    bytes[6] = (bytes[6] & 0x0f) | 0x30;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes).to_string()
}

fn table(app: &AppHandle) -> Result<rusqlite::Connection, String> {
    let conn = super::instances::db(app)?;
    conn.execute_batch("CREATE TABLE IF NOT EXISTS offline_accounts (
        id TEXT PRIMARY KEY NOT NULL,
        nickname TEXT NOT NULL UNIQUE,
        offline_uuid TEXT NOT NULL UNIQUE
    );").map_err(|e| e.to_string())?;
    Ok(conn)
}

#[tauri::command]
pub fn list_offline_accounts(app: AppHandle) -> Result<Vec<OfflineAccount>, String> {
    let conn = table(&app)?;
    let mut stmt = conn.prepare("SELECT id,nickname,offline_uuid FROM offline_accounts ORDER BY nickname COLLATE NOCASE").map_err(|e|e.to_string())?;
    let accounts: Result<Vec<OfflineAccount>, String> = stmt.query_map([], |r| Ok(OfflineAccount { id:r.get(0)?,nickname:r.get(1)?,offline_uuid:r.get(2)? }))
        .map_err(|e|e.to_string())?
        .map(|x|x.map_err(|e|e.to_string())).collect();
    accounts
}

#[tauri::command]
pub fn create_offline_account(app: AppHandle, nickname: String) -> Result<OfflineAccount, String> {
    let nickname=nickname.trim();
    if nickname.len()==0 || nickname.len()>16 || !nickname.bytes().all(|b| b.is_ascii_alphanumeric()||b==b'_') {
        return Err("Nickname: 1-16 ASCII letters, digits or underscores".into());
    }
    let account=OfflineAccount {id:Uuid::new_v4().to_string(),nickname:nickname.to_string(),offline_uuid:offline_uuid(nickname)};
    table(&app)?.execute("INSERT INTO offline_accounts (id,nickname,offline_uuid) VALUES (?1,?2,?3)",
        params![account.id,account.nickname,account.offline_uuid])
        .map_err(|e|format!("Account already exists or cannot be saved: {e}"))?;
    Ok(account)
}

#[tauri::command]
pub fn delete_offline_account(app: AppHandle, id: String) -> Result<(), String> {
    Uuid::parse_str(&id).map_err(|_| "Invalid offline account ID")?;
    let changed=table(&app)?.execute("DELETE FROM offline_accounts WHERE id=?1",params![id]).map_err(|e|e.to_string())?;
    if changed==0 {return Err("Offline account not found".into());}
    Ok(())
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test] fn deterministic_profile_uuid(){
       assert_eq!(offline_uuid("Player"),offline_uuid("Player"));
       assert_ne!(offline_uuid("Player"),offline_uuid("Player2"));
    }
}
