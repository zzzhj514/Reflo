use crate::infrastructure::{
    credentials,
    db::{conversions, reader},
};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::time::Duration;
use zip::ZipArchive;

const BASE_URL: &str = "https://mineru.net";
const MAX_DOWNLOAD_BYTES: u64 = 512 * 1024 * 1024;
const KEYCHAIN_ACCOUNT: &str = "mineru-api-token";

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MinerUPreferences {
    pub model: String,
    pub language: String,
    pub enable_ocr: bool,
    pub enable_formula: bool,
    pub enable_table: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveMinerUSettings {
    #[serde(default)]
    pub api_token: Option<String>,
    pub preferences: MinerUPreferences,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MinerUSettingsStatus {
    pub preferences: MinerUPreferences,
    pub token_configured: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownDocument {
    pub document_id: String,
    pub relative_path: String,
    pub markdown: String,
    pub processor: String,
    pub model: String,
    pub updated_at: String,
    pub asset_base_path: String,
}

struct MinerUSettings {
    api_token: String,
    preferences: MinerUPreferences,
}

#[derive(Deserialize)]
struct ApiEnvelope<T> {
    code: i64,
    msg: String,
    data: Option<T>,
}

#[derive(Deserialize)]
struct UploadData {
    batch_id: String,
    file_urls: Vec<String>,
}

#[derive(Deserialize)]
struct BatchData {
    extract_result: Vec<ExtractResult>,
}

#[derive(Deserialize)]
struct ExtractResult {
    state: String,
    #[serde(default)]
    err_msg: String,
    full_zip_url: Option<String>,
}

#[derive(Serialize)]
struct UploadRequest<'a> {
    files: [UploadFile<'a>; 1],
    model_version: &'a str,
    is_ocr: bool,
    enable_formula: bool,
    enable_table: bool,
    language: &'a str,
}

#[derive(Serialize)]
struct UploadFile<'a> {
    name: &'a str,
}

fn validate_preferences(settings: &MinerUPreferences) -> Result<(), String> {
    if !matches!(settings.model.as_str(), "vlm" | "pipeline") {
        return Err("不支持的 MinerU 模型".into());
    }
    if !matches!(
        settings.language.as_str(),
        "en" | "ch" | "ja" | "ko" | "fr" | "de" | "es"
    ) {
        return Err("不支持的文档语言".into());
    }
    Ok(())
}

fn preferences_from_record(record: conversions::MinerUPreferencesRecord) -> MinerUPreferences {
    MinerUPreferences {
        model: record.model,
        language: record.language,
        enable_ocr: record.enable_ocr,
        enable_formula: record.enable_formula,
        enable_table: record.enable_table,
    }
}

pub fn get_settings(data_dir: &Path) -> Result<MinerUSettingsStatus, String> {
    let preferences = conversions::mineru_preferences(&data_dir.join("reflo.sqlite"))
        .map(preferences_from_record)
        .map_err(|error| error.to_string())?;
    validate_preferences(&preferences)?;
    Ok(MinerUSettingsStatus {
        preferences,
        token_configured: credentials::get(KEYCHAIN_ACCOUNT)?.is_some(),
    })
}

pub fn save_settings(
    data_dir: &Path,
    mut input: SaveMinerUSettings,
) -> Result<MinerUSettingsStatus, String> {
    input.preferences.model = input.preferences.model.trim().to_owned();
    input.preferences.language = input.preferences.language.trim().to_owned();
    validate_preferences(&input.preferences)?;
    if let Some(token) = input.api_token.take() {
        let token = token.trim();
        if token.is_empty() || token.len() > 4096 {
            return Err("请填写有效的 MinerU API Token".into());
        }
        credentials::set(KEYCHAIN_ACCOUNT, token)?;
    }
    conversions::save_mineru_preferences(
        &data_dir.join("reflo.sqlite"),
        &conversions::MinerUPreferencesRecord {
            model: input.preferences.model.clone(),
            language: input.preferences.language.clone(),
            enable_ocr: input.preferences.enable_ocr,
            enable_formula: input.preferences.enable_formula,
            enable_table: input.preferences.enable_table,
        },
    )
    .map_err(|error| error.to_string())?;
    get_settings(data_dir)
}

fn https_url(value: &str, label: &str) -> Result<String, String> {
    let parsed = url::Url::parse(value).map_err(|_| format!("{label}无效"))?;
    if parsed.scheme() != "https" || parsed.host_str().is_none() {
        return Err(format!("{label}必须是 HTTPS 地址"));
    }
    Ok(value.to_owned())
}

fn document_path(data_dir: &Path, relative_path: &str) -> Result<PathBuf, String> {
    let library = data_dir
        .join("library")
        .canonicalize()
        .map_err(|_| "找不到本地文献库")?;
    let path = data_dir
        .join(relative_path)
        .canonicalize()
        .map_err(|_| "PDF 文件已丢失")?;
    if !path.starts_with(library) || !path.is_file() {
        return Err("PDF 不在本地文献库范围内".into());
    }
    Ok(path)
}

fn api_error<T>(response: ApiEnvelope<T>, action: &str) -> Result<T, String> {
    if response.code != 0 {
        return Err(format!("{action}失败：{}", response.msg));
    }
    response
        .data
        .ok_or_else(|| format!("{action}失败：响应缺少数据"))
}

fn extract_archive(bytes: &[u8], destination: &Path) -> Result<String, String> {
    let mut archive =
        ZipArchive::new(Cursor::new(bytes)).map_err(|e| format!("无法读取 MinerU 结果：{e}"))?;
    let markdown_index = (0..archive.len())
        .find(|index| {
            archive.by_index(*index).ok().is_some_and(|entry| {
                let name = entry.name().replace('\\', "/");
                name == "full.md" || name.ends_with("/full.md")
            })
        })
        .ok_or("MinerU 结果中缺少 full.md")?;
    let markdown_size = archive
        .by_index(markdown_index)
        .map_err(|e| e.to_string())?
        .size();
    if markdown_size > 50 * 1024 * 1024 {
        return Err("Markdown 文件超过 50 MB".into());
    }
    let mut markdown = String::new();
    archive
        .by_index(markdown_index)
        .map_err(|e| e.to_string())?
        .take(50 * 1024 * 1024)
        .read_to_string(&mut markdown)
        .map_err(|e| format!("读取 Markdown 失败：{e}"))?;

    let assets = destination.join("assets");
    fs::create_dir_all(&assets).map_err(|e| e.to_string())?;
    let mut used_names = HashSet::new();
    let mut extracted_bytes = markdown_size;
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(|e| e.to_string())?;
        let source_name = entry.name().replace('\\', "/");
        if !matches!(
            Path::new(&source_name)
                .extension()
                .and_then(|value| value.to_str())
                .map(str::to_ascii_lowercase)
                .as_deref(),
            Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "svg")
        ) {
            continue;
        }
        if entry.size() > 100 * 1024 * 1024 || extracted_bytes + entry.size() > MAX_DOWNLOAD_BYTES {
            return Err("MinerU 解压结果过大".into());
        }
        extracted_bytes += entry.size();
        let original_name = Path::new(&source_name)
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or("图片文件名无效")?;
        let mut file_name = original_name.to_owned();
        let mut suffix = 1;
        while !used_names.insert(file_name.clone()) {
            file_name = format!("{suffix}-{original_name}");
            suffix += 1;
        }
        let mut output = fs::File::create(assets.join(&file_name)).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry.take(100 * 1024 * 1024), &mut output)
            .map_err(|e| e.to_string())?;
        markdown = markdown.replace(&source_name, &format!("assets/{file_name}"));
    }
    Ok(markdown)
}

fn to_document(
    data_dir: &Path,
    record: conversions::MarkdownRecord,
) -> Result<MarkdownDocument, String> {
    let path = data_dir.join(&record.relative_path);
    let library = data_dir
        .join("library")
        .canonicalize()
        .map_err(|_| "找不到本地文献库")?;
    let canonical = path.canonicalize().map_err(|_| "Markdown 文件已丢失")?;
    if !canonical.starts_with(library) || !canonical.is_file() {
        return Err("Markdown 文件不在本地文献库范围内".into());
    }
    let asset_base_path = canonical
        .parent()
        .ok_or("Markdown 路径无效")?
        .to_string_lossy()
        .into_owned();
    let markdown = fs::read_to_string(canonical).map_err(|e| e.to_string())?;
    Ok(MarkdownDocument {
        document_id: record.document_id,
        relative_path: record.relative_path,
        markdown,
        processor: record.processor,
        model: record.model,
        updated_at: record.updated_at,
        asset_base_path,
    })
}

pub fn get_markdown(data_dir: &Path, paper_id: &str) -> Result<Option<MarkdownDocument>, String> {
    let document = reader::document_for_paper(&data_dir.join("reflo.sqlite"), paper_id)
        .map_err(|e| e.to_string())?
        .ok_or("这篇文献没有 PDF 文件")?;
    conversions::find_markdown(&data_dir.join("reflo.sqlite"), &document.id)
        .map_err(|e| e.to_string())?
        .map(|record| to_document(data_dir, record))
        .transpose()
}

pub fn convert(data_dir: &Path, paper_id: &str) -> Result<MarkdownDocument, String> {
    let preferences = conversions::mineru_preferences(&data_dir.join("reflo.sqlite"))
        .map(preferences_from_record)
        .map_err(|error| error.to_string())?;
    validate_preferences(&preferences)?;
    let api_token = credentials::get(KEYCHAIN_ACCOUNT)?.ok_or("请先保存 MinerU API Token")?;
    let settings = MinerUSettings {
        api_token,
        preferences,
    };
    let database = data_dir.join("reflo.sqlite");
    let document = reader::document_for_paper(&database, paper_id)
        .map_err(|e| e.to_string())?
        .ok_or("这篇文献没有 PDF 文件")?;
    let pdf_path = document_path(data_dir, &document.relative_path)?;
    let pdf = fs::read(&pdf_path).map_err(|e| e.to_string())?;
    let client = Client::builder()
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;
    let request = UploadRequest {
        files: [UploadFile {
            name: "original.pdf",
        }],
        model_version: &settings.preferences.model,
        is_ocr: settings.preferences.enable_ocr,
        enable_formula: settings.preferences.enable_formula,
        enable_table: settings.preferences.enable_table,
        language: &settings.preferences.language,
    };
    let upload_response: ApiEnvelope<UploadData> = client
        .post(format!("{BASE_URL}/api/v4/file-urls/batch"))
        .bearer_auth(&settings.api_token)
        .json(&request)
        .send()
        .map_err(|e| format!("连接 MinerU 失败：{e}"))?
        .error_for_status()
        .map_err(|e| format!("MinerU 拒绝请求：{e}"))?
        .json()
        .map_err(|e| e.to_string())?;
    let upload = api_error(upload_response, "申请上传地址")?;
    let upload_url = upload
        .file_urls
        .first()
        .ok_or_else(|| "MinerU 未返回上传地址".to_string())
        .and_then(|value| https_url(value, "上传地址"))?;
    client
        .put(upload_url)
        .body(pdf)
        .send()
        .map_err(|e| format!("上传 PDF 失败：{e}"))?
        .error_for_status()
        .map_err(|e| format!("上传 PDF 失败：{e}"))?;

    let result = (0..300)
        .find_map(|_| {
            std::thread::sleep(Duration::from_secs(3));
            let response = client
                .get(format!(
                    "{BASE_URL}/api/v4/extract-results/batch/{}",
                    upload.batch_id
                ))
                .bearer_auth(&settings.api_token)
                .send()
                .ok()?
                .error_for_status()
                .ok()?
                .json::<ApiEnvelope<BatchData>>()
                .ok()?;
            let data = api_error(response, "查询解析状态").ok()?;
            let result = data.extract_result.into_iter().next()?;
            matches!(result.state.as_str(), "done" | "failed").then_some(result)
        })
        .ok_or("MinerU 转换超时")?;
    if result.state != "done" {
        return Err(format!("MinerU 转换失败：{}", result.err_msg));
    }
    let zip_url = https_url(
        result
            .full_zip_url
            .as_deref()
            .ok_or("MinerU 未返回结果文件")?,
        "结果地址",
    )?;
    let mut response = client
        .get(zip_url)
        .send()
        .map_err(|e| format!("下载 MinerU 结果失败：{e}"))?
        .error_for_status()
        .map_err(|e| format!("下载 MinerU 结果失败：{e}"))?;
    if response
        .content_length()
        .is_some_and(|length| length > MAX_DOWNLOAD_BYTES)
    {
        return Err("MinerU 结果文件超过 512 MB".into());
    }
    let mut archive_bytes = Vec::new();
    response
        .by_ref()
        .take(MAX_DOWNLOAD_BYTES + 1)
        .read_to_end(&mut archive_bytes)
        .map_err(|e| e.to_string())?;
    if archive_bytes.len() as u64 > MAX_DOWNLOAD_BYTES {
        return Err("MinerU 结果文件超过 512 MB".into());
    }

    let final_dir = data_dir.join("library").join(&document.id).join("markdown");
    let staged_dir = data_dir
        .join("staging")
        .join(format!("markdown-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&staged_dir).map_err(|e| e.to_string())?;
    let conversion = (|| {
        let markdown = extract_archive(&archive_bytes, &staged_dir)?;
        fs::write(staged_dir.join("paper.md"), &markdown).map_err(|e| e.to_string())?;
        let backup_dir = data_dir
            .join("library")
            .join(&document.id)
            .join(format!("markdown-backup-{}", uuid::Uuid::new_v4()));
        let had_previous = final_dir.exists();
        if had_previous {
            fs::rename(&final_dir, &backup_dir).map_err(|e| e.to_string())?;
        }
        if let Err(error) = fs::rename(&staged_dir, &final_dir) {
            if had_previous {
                let _ = fs::rename(&backup_dir, &final_dir);
            }
            return Err(error.to_string());
        }
        let relative_path = format!("library/{}/markdown/paper.md", document.id);
        let record = match conversions::save_markdown(
            &database,
            &document.id,
            &relative_path,
            "mineru-v4",
            &settings.preferences.model,
        ) {
            Ok(record) => record,
            Err(error) => {
                let _ = fs::remove_dir_all(&final_dir);
                if had_previous {
                    let _ = fs::rename(&backup_dir, &final_dir);
                }
                return Err(error.to_string());
            }
        };
        if had_previous {
            let _ = fs::remove_dir_all(backup_dir);
        }
        to_document(data_dir, record)
    })();
    if staged_dir.exists() {
        let _ = fs::remove_dir_all(staged_dir);
    }
    conversion
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn extracts_markdown_and_rewrites_image_paths() {
        let cursor = Cursor::new(Vec::new());
        let mut writer = zip::ZipWriter::new(cursor);
        let options = zip::write::SimpleFileOptions::default();
        writer.start_file("full.md", options).unwrap();
        writer
            .write_all(b"# Paper\n\n![figure](images/figure.png)")
            .unwrap();
        writer.start_file("images/figure.png", options).unwrap();
        writer.write_all(b"image bytes").unwrap();
        let archive = writer.finish().unwrap().into_inner();
        let destination =
            std::env::temp_dir().join(format!("reflo-markdown-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&destination).unwrap();
        let markdown = extract_archive(&archive, &destination).unwrap();
        assert!(markdown.contains("assets/figure.png"));
        assert_eq!(
            fs::read(destination.join("assets/figure.png")).unwrap(),
            b"image bytes"
        );
        fs::remove_dir_all(destination).unwrap();
    }
}
