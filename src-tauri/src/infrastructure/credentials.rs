const SERVICE: &str = "com.zzzhj514.reflo";
const ITEM_NOT_FOUND: i32 = -25300;

#[cfg(target_os = "macos")]
pub fn get(account: &str) -> Result<Option<String>, String> {
    match security_framework::passwords::get_generic_password(SERVICE, account) {
        Ok(bytes) => String::from_utf8(bytes)
            .map(Some)
            .map_err(|_| "钥匙串中的凭据编码无效".to_string()),
        Err(error) if error.code() == ITEM_NOT_FOUND => Ok(None),
        Err(error) => Err(format!("读取 macOS 钥匙串失败：{error}")),
    }
}

#[cfg(target_os = "macos")]
pub fn set(account: &str, secret: &str) -> Result<(), String> {
    security_framework::passwords::set_generic_password(SERVICE, account, secret.as_bytes())
        .map_err(|error| format!("写入 macOS 钥匙串失败：{error}"))
}

#[cfg(not(target_os = "macos"))]
pub fn get(_account: &str) -> Result<Option<String>, String> {
    Err("当前系统尚未实现安全凭据存储".into())
}

#[cfg(not(target_os = "macos"))]
pub fn set(_account: &str, _secret: &str) -> Result<(), String> {
    Err("当前系统尚未实现安全凭据存储".into())
}
