use crate::infrastructure::db::{conversions, credentials, reader, translations};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

const MAX_INPUT_CHARS: usize = 12_000;
const MARKDOWN_CHUNK_CHARS: usize = 6_000;
const MAX_MARKDOWN_CHARS: usize = 500_000;

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
#[serde(rename_all = "camelCase")]
pub struct MarkdownTranslationDocument {
    pub document_id: String,
    pub relative_path: String,
    pub markdown: String,
    pub provider: String,
    pub model: String,
    pub target_language: String,
    pub updated_at: String,
    pub asset_base_path: String,
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

pub struct ModelCompletion {
    pub content: String,
    pub provider: String,
    pub model: String,
}

fn credential_name(provider: &str) -> Result<&'static str, String> {
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
    credential_name(&preferences.provider)?;
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
    let database = data_dir.join("reflo.sqlite");
    let mut preferences = translations::settings(&database)
        .map(from_record)
        .map_err(|error| error.to_string())?;
    validate(&mut preferences)?;
    let account = credential_name(&preferences.provider)?;
    let configured_providers = ["openai", "deepseek", "custom"]
        .into_iter()
        .map(|provider| {
            credentials::get(&database, credential_name(provider)?)
                .map_err(|error| error.to_string())
                .map(|key| key.map(|_| provider.to_owned()))
        })
        .collect::<Result<Vec<_>, String>>()?
        .into_iter()
        .flatten()
        .collect();
    Ok(TranslationSettingsStatus {
        preferences,
        key_configured: credentials::get(&database, account)
            .map_err(|error| error.to_string())?
            .is_some(),
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
        credentials::set(
            &data_dir.join("reflo.sqlite"),
            credential_name(&input.preferences.provider)?,
            api_key,
        )
        .map_err(|error| error.to_string())?;
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

fn resolved_target_code(configured: &str, text: &str) -> &'static str {
    if configured != "auto" {
        return match configured {
            "zh-CN" => "zh-CN",
            "en" => "en",
            "ja" => "ja",
            "ko" => "ko",
            "fr" => "fr",
            "de" => "de",
            "es" => "es",
            _ => "zh-CN",
        };
    }
    if text
        .chars()
        .any(|character| ('\u{3400}'..='\u{9fff}').contains(&character))
    {
        "en"
    } else {
        "zh-CN"
    }
}

fn completion_url(base_url: &str) -> String {
    if base_url.ends_with("/chat/completions") {
        base_url.to_owned()
    } else {
        format!("{}/chat/completions", base_url.trim_end_matches('/'))
    }
}

fn request_translation(
    client: &Client,
    preferences: &TranslationPreferences,
    api_key: &str,
    text: &str,
    target: &str,
    markdown: bool,
) -> Result<String, String> {
    let format_instruction = if markdown {
        " Preserve all Markdown structure exactly, including headings, lists, tables, links, image paths, HTML, code blocks, inline code, and LaTeX delimiters. Do not translate URLs, file paths, code, or formulas."
    } else {
        " Preserve technical terminology, citations, formulas, and paragraph breaks."
    };
    let instruction = format!(
        "Translate the user's academic text into {target}.{format_instruction} Return only the translation without commentary."
    );
    request_chat_completion(client, preferences, api_key, &instruction, text, "翻译")
}

fn request_chat_completion(
    client: &Client,
    preferences: &TranslationPreferences,
    api_key: &str,
    system: &str,
    user: &str,
    operation: &str,
) -> Result<String, String> {
    let request = ChatRequest {
        model: &preferences.model,
        messages: [
            ChatMessage {
                role: "system",
                content: system,
            },
            ChatMessage {
                role: "user",
                content: user,
            },
        ],
        stream: false,
    };
    let response = client
        .post(completion_url(&preferences.base_url))
        .bearer_auth(api_key)
        .json(&request)
        .send()
        .map_err(|error| format!("连接{operation}模型失败：{error}"))?;
    let status = response.status();
    if !status.is_success() {
        let details: String = response
            .text()
            .unwrap_or_default()
            .chars()
            .take(2000)
            .collect();
        return Err(format!("{operation}模型返回 {status}：{details}"));
    }
    response
        .json::<ChatResponse>()
        .map_err(|error| format!("解析{operation}响应失败：{error}"))?
        .choices
        .into_iter()
        .next()
        .and_then(|choice| choice.message.content)
        .map(|content| content.trim().to_owned())
        .filter(|content| !content.is_empty())
        .ok_or_else(|| format!("{operation}模型没有返回文字"))
}

fn client() -> Result<Client, String> {
    Client::builder()
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|error| error.to_string())
}

pub fn complete_model(
    data_dir: &Path,
    system: &str,
    user: &str,
) -> Result<ModelCompletion, String> {
    let mut preferences = translations::settings(&data_dir.join("reflo.sqlite"))
        .map(from_record)
        .map_err(|error| error.to_string())?;
    validate(&mut preferences)?;
    let api_key = credentials::get(
        &data_dir.join("reflo.sqlite"),
        credential_name(&preferences.provider)?,
    )
    .map_err(|error| error.to_string())?
    .ok_or("请先在翻译设置中保存 API Key")?;
    let content = request_chat_completion(
        &client()?,
        &preferences,
        &api_key,
        system,
        user,
        "Paper Tree 生成",
    )?;
    Ok(ModelCompletion {
        content,
        provider: preferences.provider,
        model: preferences.model,
    })
}

pub fn translate(data_dir: &Path, text: &str) -> Result<TranslationResult, String> {
    let mut preferences = translations::settings(&data_dir.join("reflo.sqlite"))
        .map(from_record)
        .map_err(|error| error.to_string())?;
    validate(&mut preferences)?;
    let api_key = credentials::get(
        &data_dir.join("reflo.sqlite"),
        credential_name(&preferences.provider)?,
    )
    .map_err(|error| error.to_string())?
    .ok_or("请先在翻译设置中保存 API Key")?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("没有可翻译的文字".into());
    }
    let source_text: String = trimmed.chars().take(MAX_INPUT_CHARS).collect();
    let truncated = source_text.chars().count() < trimmed.chars().count();
    let target = resolved_target(&preferences.target_language, &source_text);
    let target_code = resolved_target_code(&preferences.target_language, &source_text).to_owned();
    let translated_text = request_translation(
        &client()?,
        &preferences,
        &api_key,
        &source_text,
        target,
        false,
    )?;
    Ok(TranslationResult {
        source_text,
        translated_text,
        target_language: target_code,
        provider: preferences.provider,
        model: preferences.model,
        truncated,
    })
}

fn markdown_chunks(markdown: &str) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    for block in markdown.split_inclusive("\n\n") {
        let block_chars = block.chars().count();
        if !current.is_empty() && current.chars().count() + block_chars > MARKDOWN_CHUNK_CHARS {
            chunks.push(std::mem::take(&mut current));
        }
        if block_chars <= MARKDOWN_CHUNK_CHARS {
            current.push_str(block);
            continue;
        }
        let mut long = String::new();
        for character in block.chars() {
            long.push(character);
            if long.chars().count() >= MARKDOWN_CHUNK_CHARS {
                chunks.push(std::mem::take(&mut long));
            }
        }
        current.push_str(&long);
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

fn translation_path(data_dir: &Path, relative_path: &str) -> Result<PathBuf, String> {
    let library = data_dir
        .join("library")
        .canonicalize()
        .map_err(|_| "找不到本地文献库")?;
    let path = data_dir
        .join(relative_path)
        .canonicalize()
        .map_err(|_| "译文文件已丢失")?;
    if !path.starts_with(library) || !path.is_file() {
        return Err("译文文件不在本地文献库范围内".into());
    }
    Ok(path)
}

fn to_markdown_document(
    data_dir: &Path,
    record: translations::MarkdownTranslationRecord,
) -> Result<MarkdownTranslationDocument, String> {
    let path = translation_path(data_dir, &record.relative_path)?;
    let markdown = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    Ok(MarkdownTranslationDocument {
        document_id: record.document_id,
        relative_path: record.relative_path,
        markdown,
        provider: record.provider,
        model: record.model,
        target_language: record.target_language,
        updated_at: record.updated_at,
        asset_base_path: path
            .parent()
            .ok_or("译文路径无效")?
            .to_string_lossy()
            .into_owned(),
    })
}

pub fn get_markdown_translation(
    data_dir: &Path,
    paper_id: &str,
) -> Result<Option<MarkdownTranslationDocument>, String> {
    let database = data_dir.join("reflo.sqlite");
    let document = reader::document_for_paper(&database, paper_id)
        .map_err(|error| error.to_string())?
        .ok_or("这篇文献没有 PDF 文件")?;
    translations::find_markdown_translation(&database, &document.id)
        .map_err(|error| error.to_string())?
        .map(|record| to_markdown_document(data_dir, record))
        .transpose()
}

pub fn translate_markdown(
    data_dir: &Path,
    paper_id: &str,
) -> Result<MarkdownTranslationDocument, String> {
    let database = data_dir.join("reflo.sqlite");
    let document = reader::document_for_paper(&database, paper_id)
        .map_err(|error| error.to_string())?
        .ok_or("这篇文献没有 PDF 文件")?;
    let source = conversions::find_markdown(&database, &document.id)
        .map_err(|error| error.to_string())?
        .ok_or("请先将 PDF 转换为 Markdown")?;
    let source_path = data_dir
        .join(&source.relative_path)
        .canonicalize()
        .map_err(|_| "Markdown 文件已丢失")?;
    let library = data_dir
        .join("library")
        .canonicalize()
        .map_err(|_| "找不到本地文献库")?;
    if !source_path.starts_with(&library) || !source_path.is_file() {
        return Err("Markdown 文件不在本地文献库范围内".into());
    }
    let markdown = fs::read_to_string(&source_path).map_err(|error| error.to_string())?;
    if markdown.chars().count() > MAX_MARKDOWN_CHARS {
        return Err("Markdown 超过 500,000 个字符，请分章节翻译".into());
    }
    let mut preferences = translations::settings(&database)
        .map(from_record)
        .map_err(|error| error.to_string())?;
    validate(&mut preferences)?;
    let api_key = credentials::get(
        &data_dir.join("reflo.sqlite"),
        credential_name(&preferences.provider)?,
    )
    .map_err(|error| error.to_string())?
    .ok_or("请先在翻译设置中保存 API Key")?;
    let target_code = resolved_target_code(&preferences.target_language, &markdown);
    let target = resolved_target(target_code, &markdown);
    let client = client()?;
    let chunks = markdown_chunks(&markdown);
    let mut translated_chunks = Vec::with_capacity(chunks.len());
    for (index, chunk) in chunks.iter().enumerate() {
        translated_chunks.push(
            request_translation(&client, &preferences, &api_key, chunk, target, true).map_err(
                |error| format!("第 {}/{} 段翻译失败：{error}", index + 1, chunks.len()),
            )?,
        );
    }
    let translated = translated_chunks.join("\n\n");
    let parent = source_path.parent().ok_or("Markdown 路径无效")?;
    let output = parent.join("translated.md");
    let staged = parent.join(format!("translated-{}.tmp", uuid::Uuid::new_v4()));
    fs::write(&staged, translated).map_err(|error| error.to_string())?;
    fs::rename(&staged, &output).map_err(|error| error.to_string())?;
    let relative_path = Path::new(&source.relative_path)
        .parent()
        .ok_or("Markdown 相对路径无效")?
        .join("translated.md")
        .to_string_lossy()
        .into_owned();
    let record = translations::save_markdown_translation(
        &database,
        &translations::MarkdownTranslationRecord {
            document_id: document.id,
            relative_path,
            provider: preferences.provider,
            model: preferences.model,
            target_language: target_code.to_owned(),
            updated_at: String::new(),
        },
    )
    .map_err(|error| error.to_string())?;
    to_markdown_document(data_dir, record)
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
        assert_eq!(
            markdown_chunks("# A\n\nText\n\n").concat(),
            "# A\n\nText\n\n"
        );
        let long = "x".repeat(MARKDOWN_CHUNK_CHARS * 2 + 13);
        let chunks = markdown_chunks(&long);
        assert_eq!(chunks.concat(), long);
        assert!(chunks
            .iter()
            .all(|chunk| chunk.chars().count() <= MARKDOWN_CHUNK_CHARS));
    }
}
