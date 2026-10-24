//! Microsoft device-code sign-in for a Minecraft Java account.
//! Requires a developer-owned, Minecraft-Services-approved public Client ID.
//! Tokens are held in memory ONLY, never exposed to the React webview or saved.
//! Do not log bearer tokens, device codes, requests, or responses.
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Clone)]
struct Pending {
    client_id: String,
    device_code: String,
    expires_at: Instant,
    interval: u64,
    next_poll: Instant,
}
#[derive(Clone)]
struct Session {
    name: String,
    id: String,
    token: String,
    expires_at: Instant,
}
static PENDING: OnceLock<Mutex<Option<Pending>>> = OnceLock::new();
static SESSION: OnceLock<Mutex<Option<Session>>> = OnceLock::new();
fn pending() -> &'static Mutex<Option<Pending>> { PENDING.get_or_init(|| Mutex::new(None)) }
fn session() -> &'static Mutex<Option<Session>> { SESSION.get_or_init(|| Mutex::new(None)) }

#[derive(Deserialize)]
struct DeviceGrant {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    #[serde(default="default_interval")]
    interval: u64,
}
fn default_interval() -> u64 { 5 }

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct DeviceChallenge {
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}
#[derive(Clone, Serialize)]
#[serde(rename_all="camelCase")]
pub struct MicrosoftAccount {
    pub name: String,
    pub id: String,
}
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct PollStatus {
    status: String,
    account: Option<MicrosoftAccount>,
    interval: u64,
}
fn http() -> Result<Client, String> {
    Client::builder()
        .user_agent("FRE-LAUNCH-Microsoft-Login/0.5.7")
        .timeout(Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::none())
        .build().map_err(|_| "Не вдалося ініціалізувати захищене з'єднання".to_string())
}
fn pending_status(interval:u64) -> PollStatus {
    PollStatus{status:"pending".into(), account:None, interval}
}
fn safe_profile(name: &str, id: &str) -> bool {
    (1..=16).contains(&name.len())
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        && id.len()==32 && id.bytes().all(|b|b.is_ascii_hexdigit())
}

#[tauri::command]
pub async fn microsoft_begin_login(client_id:String) -> Result<DeviceChallenge,String> {
    let id = Uuid::parse_str(client_id.trim()).map_err(|_| "Потрібен справжній UUID Microsoft Application Client ID".to_string())?;
    let client_id = id.to_string();
    let client = http()?;
    let response = client.post("https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode")
        .form(&[("client_id",client_id.as_str()),("scope","XboxLive.signin offline_access")])
        .send().await.map_err(|_| "Не вдалося зв'язатися з Microsoft".to_string())?;
    if !response.status().is_success(){
        return Err(format!("Microsoft відхилив запит входу (HTTP {}). Перевір Client ID і підтримку персональних акаунтів.", response.status()));
    }
    let grant:DeviceGrant = response.json().await.map_err(|_|"Некоректна відповідь Microsoft".to_string())?;
    if grant.user_code.is_empty() || grant.device_code.is_empty() || grant.expires_in==0 {
        return Err("Microsoft повернув неповний код підтвердження".into());
    }
    // Never open an arbitrary verification page returned by an external service.
    let verify = reqwest::Url::parse(&grant.verification_uri).map_err(|_|"Некоректна адреса Microsoft".to_string())?;
    if verify.scheme()!="https" || !matches!(verify.host_str(),Some("microsoft.com"|"www.microsoft.com"|"login.microsoftonline.com")){
        return Err("Microsoft повернув неочікувану адресу підтвердження".into());
    }
    let interval=grant.interval.max(5).min(60);
    *pending().lock().map_err(|_|"Стан авторизації недоступний")? = Some(Pending {
        client_id, device_code:grant.device_code,
        expires_at:Instant::now()+Duration::from_secs(grant.expires_in.min(1800)),
        interval,next_poll:Instant::now()+Duration::from_secs(interval),
    });
    Ok(DeviceChallenge{
        user_code:grant.user_code, verification_uri:grant.verification_uri,
        expires_in:grant.expires_in,interval
    })
}

async fn exchange_for_minecraft(client:&Client, access:&str)-> Result<Session,String> {
    let xbl = client.post("https://user.auth.xboxlive.com/user/authenticate")
        .json(&json!({"Properties":{"AuthMethod":"RPS","SiteName":"user.auth.xboxlive.com",
            "RpsTicket":format!("d={access}")},
            "RelyingParty":"http://auth.xboxlive.com","TokenType":"JWT"}))
        .send().await.map_err(|_|"Не вдалося зв'язатися з Xbox Live")?;
    if !xbl.status().is_success(){return Err(format!("Xbox Live відхилив вхід (HTTP {})",xbl.status()));}
    let xbl:Value=xbl.json().await.map_err(|_|"Некоректна відповідь Xbox Live")?;
    let xbl_token=xbl["Token"].as_str().filter(|s|!s.is_empty()).ok_or("Xbox Live не повернув токен")?;

    let xsts=client.post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .json(&json!({"Properties":{"SandboxId":"RETAIL","UserTokens":[xbl_token]},
            "RelyingParty":"rp://api.minecraftservices.com/","TokenType":"JWT"}))
        .send().await.map_err(|_|"Не вдалося зв'язатися з Xbox XSTS")?;
    if !xsts.status().is_success(){return Err(format!("Xbox XSTS відхилив вхід (HTTP {}). Перевір налаштування Xbox-профілю.",xsts.status()));}
    let xsts:Value=xsts.json().await.map_err(|_|"Некоректна відповідь Xbox XSTS")?;
    let xsts_token=xsts["Token"].as_str().filter(|s|!s.is_empty()).ok_or("Xbox XSTS не повернув токен")?;
    let uhs=xsts["DisplayClaims"]["xui"][0]["uhs"].as_str().filter(|s|!s.is_empty()).ok_or("Xbox XSTS не повернув ідентифікатор користувача")?;

    let mc=client.post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .json(&json!({"identityToken":format!("XBL3.0 x={uhs};{xsts_token}"),"ensureLegacyEnabled":true}))
        .send().await.map_err(|_|"Не вдалося зв'язатися з Minecraft Services")?;
    if mc.status().as_u16()==403 {
        return Err("Minecraft Services відхилив застосунок (403). Для власного Client ID потрібен дозвіл Minecraft App ID; сам вхід у Microsoft цього не замінює.".into());
    }
    if !mc.status().is_success(){return Err(format!("Minecraft Services відхилив вхід (HTTP {})",mc.status()));}
    let mc:Value=mc.json().await.map_err(|_|"Некоректна відповідь Minecraft Services")?;
    let token=mc["access_token"].as_str().filter(|s|!s.is_empty()).ok_or("Minecraft Services не повернув токен")?;
    let expiry=mc["expires_in"].as_u64().unwrap_or(3600).min(86400);
    let profile=client.get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(token).send().await.map_err(|_|"Не вдалося отримати Minecraft-профіль")?;
    if profile.status().as_u16()==404 {
        return Err("У цьому Microsoft-акаунті немає доступного профілю Minecraft Java Edition.".into());
    }
    if !profile.status().is_success(){return Err(format!("Minecraft-профіль недоступний (HTTP {})",profile.status()));}
    let profile:Value=profile.json().await.map_err(|_|"Некоректна відповідь профілю Minecraft")?;
    let name=profile["name"].as_str().ok_or("У Minecraft-профілі немає імені")?;
    let id=profile["id"].as_str().ok_or("У Minecraft-профілі немає UUID")?;
    if !safe_profile(name,id){return Err("Minecraft повернув некоректні дані профілю".into());}
    Ok(Session{name:name.into(),id:id.into(),token:token.into(),
        expires_at:Instant::now()+Duration::from_secs(expiry.saturating_sub(120).max(60))})
}

#[tauri::command]
pub async fn microsoft_poll_login() -> Result<PollStatus,String> {
    let state={
        let guard=pending().lock().map_err(|_|"Стан авторизації недоступний")?;
        guard.clone().ok_or("Спочатку натисни «Увійти через Microsoft»")?
    };
    if Instant::now()>=state.expires_at {
        *pending().lock().map_err(|_|"Стан авторизації недоступний")?=None;
        return Err("Час коду Microsoft минув. Почни вхід знову.".into());
    }
    if Instant::now()<state.next_poll {return Ok(pending_status(state.interval));}
    {
        let mut guard=pending().lock().map_err(|_|"Стан авторизації недоступний")?;
        if let Some(g)=guard.as_mut(){g.next_poll=Instant::now()+Duration::from_secs(g.interval);}
    }
    let client=http()?;
    let response=client.post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
        .form(&[("grant_type","urn:ietf:params:oauth:grant-type:device_code"),
            ("client_id",state.client_id.as_str()),("device_code",state.device_code.as_str())])
        .send().await.map_err(|_|"Microsoft недоступний під час перевірки коду")?;
    let success=response.status().is_success();
    let body:Value=response.json().await.map_err(|_|"Некоректна відповідь Microsoft")?;
    if !success {
        match body["error"].as_str().unwrap_or("") {
            "authorization_pending"=>return Ok(pending_status(state.interval)),
            "slow_down"=>{
                let mut guard=pending().lock().map_err(|_|"Стан авторизації недоступний")?;
                if let Some(g)=guard.as_mut(){
                    g.interval=(g.interval+5).min(60);
                    g.next_poll=Instant::now()+Duration::from_secs(g.interval);
                    return Ok(pending_status(g.interval));
                }
                return Err("Сесію Microsoft скасовано".into());
            },
            _=>{
                *pending().lock().map_err(|_|"Стан авторизації недоступний")?=None;
                return Err("Microsoft відхилив код або термін його дії завершився. Почни вхід знову.".into());
            }
        }
    }
    *pending().lock().map_err(|_|"Стан авторизації недоступний")?=None;
    let access=body["access_token"].as_str().filter(|s|!s.is_empty()).ok_or("Microsoft не повернув токен")?;
    let live=exchange_for_minecraft(&client,access).await?;
    let account=MicrosoftAccount{name:live.name.clone(),id:live.id.clone()};
    *session().lock().map_err(|_|"Стан авторизації недоступний")?=Some(live);
    Ok(PollStatus{status:"connected".into(),account:Some(account),interval:0})
}

#[tauri::command]
pub fn microsoft_current_account() -> Result<Option<MicrosoftAccount>,String> {
    let guard=session().lock().map_err(|_|"Стан авторизації недоступний")?;
    Ok(guard.as_ref().filter(|s|Instant::now()<s.expires_at)
        .map(|s|MicrosoftAccount{name:s.name.clone(),id:s.id.clone()}))
}

#[tauri::command]
pub fn microsoft_logout() -> Result<(),String> {
    *session().lock().map_err(|_|"Стан авторизації недоступний")?=None;
    *pending().lock().map_err(|_|"Стан авторизації недоступний")?=None;
    Ok(())
}

/// Only Rust launch code can read access tokens. They must never enter JS.
pub(super) fn launch_credentials(expected_name:&str)->Result<(String,String),String>{
    let guard=session().lock().map_err(|_|"Стан авторизації недоступний")?;
    let current=guard.as_ref().ok_or("Увійди через Microsoft перед запуском")?;
    if Instant::now()>=current.expires_at {return Err("Термін сесії Microsoft закінчився. Увійди ще раз.".into());}
    if current.name!=expected_name{return Err("Активний Microsoft-профіль не відповідає обраному імені.".into());}
    Ok((current.id.clone(),current.token.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn validate_minecraft_profile(){
        assert!(safe_profile("Player123","aabbccddeeff00112233445566778899"));
        assert!(!safe_profile("../bad","aabbccddeeff00112233445566778899"));
        assert!(!safe_profile("Player","../../bad"));
    }
}
