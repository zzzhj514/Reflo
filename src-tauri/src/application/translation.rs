use crate::infrastructure::{credentials, db::translations};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

const MAX_INPUT_CHARS: usize = 12_000;

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationPreferences {
    pub provider: String,
    pub base_url: String,
    pub model: String,
    pub target_language: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveTranslationSettings {
    pub preferences: TranslationPreferences,
    #[serde(default)]
    pub api_key: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationSettingsStatus {
    pub preferences: TranslationPreferences,
    pub key_configured: bool,
    pub configured_providers: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationResult {
    pub source_text: String,
    pub translated_text: String,
    pub target_language: String,
    pub provider: String,
    pub model: String,
    pub truncated: bool,
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: [ChatMessage<'a>; 2],
    stream: bool,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
}

#[derive(Deserialize)]
struct ChatResponseMessage {
    content: Option<String>,
}

fn keychain_account(provider: &str) -> Result<&'static str, String> {
    match provider {
        "openai" => Ok("translation-openai-api-key"),
        "deepseek" => Ok("translation-deepseek-api-key"),
        "custom" => Ok("translation-custom-api-key"),
        _ => Err("不支持的翻译服务商".into()),
    }
}

fn validate(preferences: &mut TranslationPreferences) -> Result<(), String> {
    preferences.provider = preferences.provider.trim().to_owned();
    preferences.base_url = preferences.base_url.trim().trim_end_matches('/').to_owned();
    preferences.model = preferences.model.trim().to_owned();
    preferences.target_language = preferences.target_language.trim().to_owned();
    keychain_account(&preferences.provider)?;
    if preferences.model.is_empty() || preferences.model.chars().count() > 200 {
        return Err("请填写有效的模型名称".into());
    }
    if !matches!(
        preferences.target_language.as_str(),
        "auto" | "zh-CN" | "en" | "ja" | "ko" | "fr" | "de" | "es"
    ) {
        return Err("不支持的目标语言".into());
    }
    let url = url::Url::parse(&preferences.base_url).map_err(|_| "API Base URL 无效")?;
    let local_http =
        url.scheme() == "http" && matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "::1"));
    if !(url.scheme() == "https" || local_http) || url.host_str().is_none() {
        return Err("云端 API 必须使用 HTTPS；HTTP 仅允许本机地址".into());
    }
    Ok(())
}

fn from_record(record: translations::TranslationSettingsRecord) -> TranslationPreferences {
    TranslationPreferences {
        provider: record.provider,
        base_url: record.base_url,
        model: record.model,
        target_language: record.target_language,
    }
}

pub fn get_settings(data_dir: &Path) -> Result<TranslationSettingsStatus, String> {
    let mut preferences = translations::settings(&data_dir.join("reflo.sqlite"))
        .map(from_record)
        .map_err(|error| error.to_string())?;
    validate(&mut preferences)?;
    let account = keychain_account(&preferences.provider)?;
    let configured_providers = ["openai", "deepseek", "custom"]
        .into_iter()
        .map(|provider| {
            credentials::get(keychain_account(provider)?)
                .map(|key| key.map(|_| provider.to_owned()))
        })
        .collect::<Result<Vec<_>, String>>()?
        .into_iter()
        .flatten()
        .collect();
    Ok(TranslationSettingsStatus {
        preferences,
        key_configured: credentials::get(account)?.is_some(),
        configured_providers,
    })
}

pub fn save_settings(
    data_dir: &Path,
    mut input: SaveTranslationSettings,
) -> Result<TranslationSettingsStatus, String> {
    validate(&mut input.preferences)?;
    if let Some(api_key) = input.api_key.take() {
        let api_key = api_key.trim();
        if api_key.is_empty() || api_key.len() > 4096 {
            return Err("请填写有效的 API Key".into());
        }
        credentials::set(keychain_account(&input.preferences.provider)?, api_key)?;
    }
    translations::save_settings(
        &data_dir.join("reflo.sqlite"),
        &translations::TranslationSettingsRecord {
            provider: input.preferences.provider.clone(),
            base_url: input.preferences.base_url.clone(),
            model: input.preferences.model.clone(),
            target_language: input.preferences.target_language.clone(),
        },
    )
    .map_err(|error| error.to_string())?;
    get_settings(data_dir)
}

fn resolved_target(configured: &str, text: &str) -> &'static str {
    if configured != "auto" {
        return match configured {
            "zh-CN" => "Simplified Chinese",
            "en" => "English",
            "ja" => "Japanese",
            "ko" => "Korean",
            "fr" => "French",
            "de" => "German",
            "es" => "Spanish",
            _ => "Simplified Chinese",
        };
    }
    if text
        .chars()
        .any(|character| ('\u{3400}'..='\u{9fff}').contains(&character))
    {
        "English"
    } else {
        "Simplified Chinese"
    }
}

fn completion_url(base_url: &str) -> String {
    if base_url.ends_with("/chat/completions") {
        base_url.to_owned()
    } else {
        format!("{}/chat/completions", base_url.trim_end_matches('/'))
    }
}

pub fn translate(data_dir: &Path, text: &str) -> Result<TranslationResult, String> {
    let mut preferences = translations::settings(&data_dir.join("reflo.sqlite"))
        .map(from_record)
        .map_err(|error| error.to_string())?;
    validate(&mut preferences)?;
    let api_key = credentials::get(keychain_account(&preferences.provider)?)?
        .ok_or("请先在翻译设置中保存 API Key")?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("没有可翻译的文字".into());
    }
    let source_text: String = trimmed.chars().take(MAX_INPUT_CHARS).collect();
    let truncated = source_text.chars().count() < trimmed.chars().count();
    let target = resolved_target(&preferences.target_language, &source_text);
    let instruction = format!(
        "Translate the user's academic text into {target}. Preserve technical terminology, citations, formulas, and paragraph breaks. Return only the translation without commentary."
    );
    let request = ChatRequest {
        model: &preferences.model,
        messages: [
            ChatMessage {
                role: "system",
                content: &instruction,
            },
            ChatMessage {
                role: "user",
                content: &source_text,
            },
        ],
        stream: false,
    };
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|error| error.to_string())?;
    let response = client
        .post(completion_url(&preferences.base_url))
        .bearer_auth(api_key)
        .json(&request)
        .send()
        .map_err(|error| format!("连接翻译模型失败：{error}"))?;
    let status = response.status();
    if !status.is_success() {
        let details: String = response
            .text()
            .unwrap_or_default()
            .chars()
            .take(2000)
            .collect();
        return Err(format!("翻译模型返回 {status}：{details}"));
    }
    let payload: ChatResponse = response
        .json()
        .map_err(|error| format!("解析翻译响应失败：{error}"))?;
    let translated_text = payload
        .choices
        .into_iter()
        .next()
        .and_then(|choice| choice.message.content)
        .map(|content| content.trim().to_owned())
        .filter(|content| !content.is_empty())
        .ok_or("翻译模型没有返回文字")?;
    Ok(TranslationResult {
        source_text,
        translated_text,
        target_language: target.to_owned(),
        provider: preferences.provider,
        model: preferences.model,
        truncated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_compatible_endpoint_and_resolves_auto_language() {
        assert_eq!(
            completion_url("https://api.openai.com/v1"),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            completion_url("https://api.deepseek.com/chat/completions"),
            "https://api.deepseek.com/chat/completions"
        );
        assert_eq!(
            resolved_target("auto", "academic paper"),
            "Simplified Chinese"
        );
        assert_eq!(resolved_target("auto", "学术论文"), "English");
        let mut local = TranslationPreferences {
            provider: "custom".into(),
            base_url: "http://localhost:11434/v1".into(),
            model: "local-model".into(),
            target_language: "auto".into(),
        };
        assert!(validate(&mut local).is_ok());
        local.base_url = "http://example.com/v1".into();
        assert!(validate(&mut local).is_err());
    }
}
