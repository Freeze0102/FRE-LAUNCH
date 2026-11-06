// FRE LAUNCH native profiles do not have Java/Theseus database instance IDs.
// These helpers reuse Minecraft's NBT codec after the Tauri layer validates the paths.
pub async fn fre_read_native_world(path: PathBuf) -> Result<World> {
    read_singleplayer_world(path).await
}
pub fn fre_validate_native_address(address: &str) -> Result<()> {
    if address.is_empty() || address.len()>255 || address.chars().any(|c| c.is_control() || c.is_whitespace()) || address.starts_with('-') {
        return Err(ErrorKind::InputError("Enter a Minecraft server address".into()).into());
    }
    parse_server_address(address)?;
    Ok(())
}
pub async fn fre_read_native_servers(path: &Path) -> Result<Vec<serde_json::Value>> {
    let servers=crate::api::instance::synced_servers::read_servers(path).await?;
    Ok(servers.iter().enumerate().filter_map(|(index,server)| {
        if server.get::<_,i8>("hidden").unwrap_or(0)!=0 {return None;}
        let address=server.get::<_,&str>("ip").ok()?;
        let name=server.get::<_,&str>("name").unwrap_or(address);
        Some(serde_json::json!({"type":"server","index":index,"name":name,"address":address,"pack_status":"prompt","display_status":"normal"}))
    }).collect())
}
pub async fn fre_edit_native_server(source:&Path,target:&Path,action:&str,name:&str,address:&str)->Result<()> {
    fre_validate_native_address(address)?;
    let mut servers=crate::api::instance::synced_servers::read_servers(source).await?;
    if action=="add" {
        if servers.len()>=500 {return Err(ErrorKind::InputError("Too many saved servers".into()).into());}
        if servers.iter().any(|s|s.get::<_,&str>("ip").ok()==Some(address)) {return Err(ErrorKind::InputError("This server is already saved".into()).into());}
        servers.push(crate::api::instance::synced_servers::server_data(name.to_owned(),address.to_owned(),None));
    } else if action=="remove" {
        let before=servers.len();
        servers.retain(|s|s.get::<_,&str>("ip").ok()!=Some(address));
        if servers.len()==before {return Err(ErrorKind::InputError("Server is no longer saved".into()).into());}
    } else {return Err(ErrorKind::InputError("Unsupported server action".into()).into());}
    crate::api::instance::synced_servers::write_servers(target,&servers).await
}
