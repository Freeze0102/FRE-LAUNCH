//! Microsoft device-code sign-in for a Minecraft Java account.
//! Requires a developer-owned, Minecraft-Services-approved public Client ID.
//! Minecraft bearer tokens stay in Rust memory. The Microsoft refresh token is saved only
//! in the Windows Credential Manager, never in JavaScript or project configuration.
//! Do not log bearer tokens, device codes, requests, or responses.
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use uuid::Uuid;

const KEYRING_SERVICE:&str="FRE-LAUNCH-Minecraft";
const KEYRING_USER:&str="MicrosoftRefreshToken";
#[derive(Serialize,Deserialize)]
struct SavedMicrosoftLogin{client_id:String,refresh_token:String}
fn credential()->Result<keyring::Entry,String>{
    keyring::Entry::new(KEYRING_SERVICE,KEYRING_USER)
        .map_err(|_|"Windows Credential Manager недоступний".to_string())
}
fn configured_client_id()->Option<String>{
    option_env!("FRE_MS_CLIENT_ID").and_then(|s|Uuid::parse_str(s.trim()).ok())
        .map(|id|id.to_string())
}
fn store_refresh(client_id:&str,refresh_token:&str)->Result<(),String>{
    let data=serde_json::to_string(&SavedMicrosoftLogin{
        client_id:client_id.into(),refresh_token:refresh_token.into()
    }).map_err(|_|"Неможливо підготувати захищену сесію")?;
    credential()?.set_password(&data)
        .map_err(|_|"Windows Credential Manager не зміг зберегти вхід".to_string())
}
fn saved_refresh()->Result<Option<SavedMicrosoftLogin>,String>{
    match credential()?.get_password(){
        Ok(text)=>serde_json::from_str(&text).map(Some)
            .map_err(|_|"Збережений токен Microsoft пошкоджений".into()),
        Err(keyring::Error::NoEntry)=>Ok(None),
        Err(_)=>Err("Не вдалося прочитати вхід із Windows Credential Manager".into())
    }
}
fn delete_refresh()->Result<(),String>{
    match credential()?.delete_credential(){
        Ok(())|Err(keyring::Error::NoEntry)=>Ok(()),
        Err(_)=>Err("Не вдалося видалити збережений Microsoft-вхід".into())
    }
}
#[tauri::command]
pub fn microsoft_login_configuration()->bool{configured_client_id().is_some()}


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
    persisted: bool,
}
fn http() -> Result<Client, String> {
    Client::builder()
        .user_agent("FRE-LAUNCH-Microsoft-Login/0.5.7")
        .timeout(Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::none())
        .build().map_err(|_| "Не вдалося ініціалізувати захищене з'єднання".to_string())
}
fn pending_status(interval:u64) -> PollStatus {
    PollStatus{status:"pending".into(), account:None, interval,persisted:false}
}
fn safe_profile(name: &str, id: &str) -> bool {
    (1..=16).contains(&name.len())
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        && id.len()==32 && id.bytes().all(|b|b.is_ascii_hexdigit())
}

/// Browser-based OAuth Authorization Code + PKCE for native Windows sign-in.
/// The Client ID must be configured as an Entra public "Mobile and desktop" app
/// with http://localhost as a redirect URI, and separately approved by Minecraft.
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct BrowserLoginResult {
    account: MicrosoftAccount,
    persisted: bool,
}
static BROWSER_LOGIN: std::sync::atomic::AtomicBool=std::sync::atomic::AtomicBool::new(false);
struct BrowserLoginGuard;
impl Drop for BrowserLoginGuard {
    fn drop(&mut self){
        BROWSER_LOGIN.store(false,std::sync::atomic::Ordering::SeqCst);
    }
}
fn login_client_id(id:Option<String>)->Result<String,String>{
    let configured=configured_client_id();
    let selected=id.as_deref().map(str::trim).filter(|s|!s.is_empty())
        .or(configured.as_deref())
        .ok_or("FRE LAUNCH потребує власний Minecraft-схвалений Client ID. Розробник повинен указати FRE_MS_CLIENT_ID.")?;
    Uuid::parse_str(selected)
        .map(|id|id.to_string())
        .map_err(|_|"Некоректний Microsoft Application Client ID".to_string())
}
fn pkce_verifier()->String{
    // 48 OS-random bytes represented using the URL-safe PKCE alphabet.
    let mut bytes=Vec::with_capacity(48);
    for _ in 0..3 {bytes.extend_from_slice(Uuid::new_v4().as_bytes());}
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}
fn pkce_challenge(verifier:&str)->String{
    use base64::Engine;
    use sha2::{Digest,Sha256};
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

#[cfg(test)]
mod browser_login_tests{
    use super::*;
    #[test]
    fn rfc7636_pkce_s256_example(){
        let verifier="dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        assert_eq!(pkce_challenge(verifier),"E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }
    #[test]
    fn generated_verifier_is_url_safe(){
        let verifier=pkce_verifier();
        assert!(verifier.len()>=43&&verifier.len()<=128);
        assert!(verifier.bytes().all(|x|x.is_ascii_alphanumeric()||x==b'-'||x==b'_'));
        assert_ne!(verifier,pkce_verifier());
    }
}
async fn read_browser_callback(listener:tokio::net::TcpListener,expected_state:&str)
    ->Result<String,String>{
    use tokio::io::{AsyncReadExt,AsyncWriteExt};
    // Overall timeout handled by the caller. Only accept a callback on loopback.
    for _ in 0..12 {
        let (mut stream,_address)=listener.accept().await
            .map_err(|_|"Не вдалося прийняти відповідь браузера".to_string())?;
        let mut bytes=[0u8;8192];
        let size=match tokio::time::timeout(Duration::from_secs(10),stream.read(&mut bytes)).await{
            Ok(Ok(size))=>size,
            _=>continue,
        };
        let line=String::from_utf8_lossy(&bytes[..size]);
        let first=line.lines().next().unwrap_or("");
        let path=first.strip_prefix("GET ").and_then(|line|line.split_whitespace().next());
        let Some(path)=path else{continue};
        // All received paths must be local relative request targets.
        if !path.starts_with('/')||path.starts_with("//"){continue;}
        let Ok(callback)=reqwest::Url::parse(&format!("http://localhost{path}")) else{continue};
        let pairs=callback.query_pairs().into_owned().collect::<std::collections::HashMap<_,_>>();
        let state_matches=pairs.get("state").is_some_and(|s|s==expected_state);
        let outcome=if state_matches{
            if let Some(code)=pairs.get("code").filter(|s|!s.is_empty()&&s.len()<4096){
                Some(Ok(code.clone()))
            }else if pairs.contains_key("error"){
                Some(Err("Microsoft-вхід скасовано або відхилено. Спробуй ще раз.".to_string()))
            }else{None}
        }else{None};
        let (status,body)=if outcome.is_some(){
            ("200 OK","<html><meta charset=\"utf-8\"><body style=\"font-family:sans-serif;text-align:center;padding:48px\"><h2>Microsoft підтвердила вхід</h2><p>Повернися до FRE LAUNCH, щоб завершити перевірку Minecraft.</p></body></html>")
        }else{
            ("400 Bad Request","<html><body>Неправильний або застарілий запит авторизації.</body></html>")
        };
        let reply=format!("HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\n\r\n{body}",body.len());
        let _=stream.write_all(reply.as_bytes()).await;
        let _=stream.shutdown().await;
        if let Some(outcome)=outcome{return outcome;}
    }
    Err("Не отримано коректної відповіді від Microsoft у браузері".into())
}
#[tauri::command]
pub async fn microsoft_browser_login(client_id:Option<String>)->Result<BrowserLoginResult,String>{
    use std::sync::atomic::Ordering;
    if BROWSER_LOGIN.compare_exchange(false,true,Ordering::SeqCst,Ordering::SeqCst).is_err(){
        return Err("Вхід через браузер уже виконується".into());
    }
    let _guard=BrowserLoginGuard;
    let id=login_client_id(client_id)?;
    let listener=tokio::net::TcpListener::bind("127.0.0.1:0").await
        .map_err(|_|"Не вдалося відкрити локальний порт для Microsoft-входу".to_string())?;
    let port=listener.local_addr().map_err(|_|"Не вдалося визначити локальний порт")?.port();
    // This URI requires http://localhost registered as a native redirect.
    let redirect=format!("http://localhost:{port}/");
    let verifier=pkce_verifier();
    let challenge=pkce_challenge(&verifier);
    let state=Uuid::new_v4().to_string();
    let mut login=reqwest::Url::parse("https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize")
        .map_err(|_|"Помилка адреси Microsoft")?;
    login.query_pairs_mut()
        .append_pair("client_id",&id)
        .append_pair("response_type","code")
        .append_pair("response_mode","query")
        .append_pair("redirect_uri",&redirect)
        .append_pair("scope","XboxLive.signin offline_access")
        .append_pair("code_challenge",&challenge)
        .append_pair("code_challenge_method","S256")
        .append_pair("state",&state)
        .append_pair("prompt","select_account");
    webbrowser::open(login.as_str()).map_err(|_|"Не вдалося відкрити системний браузер Microsoft".to_string())?;
    let code=tokio::time::timeout(Duration::from_secs(300),read_browser_callback(listener,&state))
        .await.map_err(|_|"Час входу в Microsoft завершився (5 хвилин)".to_string())??;
    let client=http()?;
    let response=client.post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
        .form(&[
            ("client_id",id.as_str()),
            ("grant_type","authorization_code"),
            ("code",code.as_str()),
            ("redirect_uri",redirect.as_str()),
            ("code_verifier",verifier.as_str()),
            ("scope","XboxLive.signin offline_access"),
        ]).send().await.map_err(|_|"Не вдалося завершити Microsoft-вхід".to_string())?;
    if !response.status().is_success(){
        return Err(format!("Microsoft відхилила авторизацію (HTTP {}). Перевір Client ID та redirect URI http://localhost.",response.status()));
    }
    let json:Value=response.json().await.map_err(|_|"Microsoft повернула некоректні токени")?;
    let access=json["access_token"].as_str().filter(|s|!s.is_empty())
        .ok_or("Microsoft не надала access token")?;
    // Xbox / XSTS / Minecraft verification is required. Microsoft login alone is insufficient.
    let live=exchange_for_minecraft(&client,access).await?;
    let account=MicrosoftAccount{name:live.name.clone(),id:live.id.clone()};
    *session().lock().map_err(|_|"Стан авторизації недоступний")?=Some(live);
    let persisted=json["refresh_token"].as_str().filter(|s|!s.is_empty())
        .is_some_and(|refresh|store_refresh(&id,refresh).is_ok());
    Ok(BrowserLoginResult{account,persisted})
}

#[tauri::command]
pub async fn microsoft_begin_login(client_id:Option<String>) -> Result<DeviceChallenge,String> {
    let configured=configured_client_id();
    let supplied=client_id.as_deref().map(str::trim).filter(|v|!v.is_empty());
    let provided=supplied.or(configured.as_deref()).ok_or(
        "У FRE LAUNCH ще немає схваленого Minecraft Client ID. Розробник повинен зареєструвати й схвалити застосунок, а потім додати FRE_MS_CLIENT_ID до збірки."
    )?;
    let id = Uuid::parse_str(provided).map_err(|_| "Потрібен справжній UUID Microsoft Application Client ID".to_string())?;
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
    let _=webbrowser::open(&grant.verification_uri);
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
    // A Windows credential storage failure does not invalidate current authentication.
    let persisted=body["refresh_token"].as_str().filter(|s|!s.is_empty())
        .is_some_and(|refresh|store_refresh(&state.client_id,refresh).is_ok());
    Ok(PollStatus{status:"connected".into(),account:Some(account),interval:0,persisted})
}

#[tauri::command]
pub async fn microsoft_restore_login()->Result<Option<MicrosoftAccount>,String>{
    if let Some(existing)=microsoft_current_account()?{return Ok(Some(existing));}
    let Some(saved)=saved_refresh()? else{return Ok(None);};
    let client=http()?;
    let response=client.post("https://login.microsoftonline.com/consumers/oauth2/v2.0/token")
        .form(&[
            ("client_id",saved.client_id.as_str()),
            ("grant_type","refresh_token"),
            ("refresh_token",saved.refresh_token.as_str()),
            ("scope","XboxLive.signin offline_access"),
        ]).send().await.map_err(|_|"Microsoft недоступний для відновлення сесії")?;
    if !response.status().is_success(){
        if response.status().as_u16()==400||response.status().as_u16()==401{
            let _=delete_refresh();
        }
        return Err("Не вдалося відновити Microsoft-вхід. Увійди ще раз.".into());
    }
    let json:Value=response.json().await.map_err(|_|"Неправильна відповідь Microsoft")?;
    let access=json["access_token"].as_str().ok_or("Не вистачає Microsoft-токена")?;
    let restored=exchange_for_minecraft(&client,access).await?;
    let account=MicrosoftAccount{name:restored.name.clone(),id:restored.id.clone()};
    *session().lock().map_err(|_|"Стан авторизації недоступний")?=Some(restored);
    if let Some(rotated)=json["refresh_token"].as_str(){let _=store_refresh(&saved.client_id,rotated);}
    Ok(Some(account))
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
    delete_refresh()?;
    Ok(())
}

/// Official Mojang skins endpoint. The bearer token never reaches React.
#[tauri::command]
pub async fn microsoft_upload_skin(png_base64:String, variant:String) -> Result<String,String> {
    use base64::Engine;
    let variant=match variant.as_str(){
        "classic"=>"classic","slim"=>"slim",
        _=>return Err("Обери модель скіна Classic або Slim".into())
    };
    // Hard cap before decoding so a malformed browser payload cannot exhaust RAM.
    if png_base64.len()>2_800_000{return Err("Скін завеликий: максимум 2 МБ PNG".into());}
    let png=base64::engine::general_purpose::STANDARD.decode(&png_base64)
        .map_err(|_|"Неправильне кодування PNG".to_string())?;
    if png.len()<24||png.len()>2*1024*1024||!png.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err("Некоректний файл PNG".into());
    }
    if &png[12..16]!=b"IHDR"{return Err("У PNG відсутній заголовок зображення".into());}
    let width=u32::from_be_bytes(png[16..20].try_into().unwrap());
    let height=u32::from_be_bytes(png[20..24].try_into().unwrap());
    if width!=64||!(height==32||height==64){
        return Err("Minecraft Services приймає скіни 64×64 або 64×32. Зменш HD-скін перед завантаженням.".into());
    }
    let _=microsoft_restore_login().await?;
    let token={
        let guard=session().lock().map_err(|_|"Стан авторизації недоступний")?;
        let current=guard.as_ref().ok_or("Спочатку увійди до Minecraft через Microsoft")?;
        if Instant::now()>=current.expires_at{return Err("Термін Microsoft-сесії завершився. Увійди знову.".into());}
        current.token.clone()
    };
    let form=reqwest::multipart::Form::new()
        .text("variant",variant.to_string())
        .part("file",reqwest::multipart::Part::bytes(png)
            .file_name("fre-skin.png")
            .mime_str("image/png").map_err(|_|"Неможливо створити PNG-запит")?);
    let response=http()?.post("https://api.minecraftservices.com/minecraft/profile/skins")
        .bearer_auth(token)
        .multipart(form)
        .send().await.map_err(|_|"Не вдалося завантажити скін до Minecraft Services")?;
    if !response.status().is_success(){
        return Err(format!("Minecraft Services не прийняв скін (HTTP {}). Переконайся, що акаунт ліцензійний та сесія активна.",response.status()));
    }
    Ok("Скін завантажено до Minecraft-профілю. У грі може знадобитися перепідключення.".into())
}

#[tauri::command]
pub async fn microsoft_reset_skin() -> Result<String,String> {
    let _=microsoft_restore_login().await?;
    let token={
        let guard=session().lock().map_err(|_|"Стан авторизації недоступний")?;
        let current=guard.as_ref().ok_or("Увійди до Minecraft через Microsoft")?;
        if Instant::now()>=current.expires_at{return Err("Microsoft-сесія закінчилася".into());}
        current.token.clone()
    };
    let response=http()?.delete("https://api.minecraftservices.com/minecraft/profile/skins/active")
        .bearer_auth(token).send().await.map_err(|_|"Не вдалося відновити стандартний скін")?;
    if !response.status().is_success(){
        return Err(format!("Minecraft Services відхилив відновлення скіна (HTTP {})",response.status()));
    }
    Ok("Minecraft-профіль повернуто до стандартного скіна".into())
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
