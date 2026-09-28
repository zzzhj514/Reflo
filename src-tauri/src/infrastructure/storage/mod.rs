use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use uuid::Uuid;

pub struct StagedPdf {
    pub document_id: String,
    pub original_name: String,
    pub staged_path: PathBuf,
    pub sha256: String,
}

pub fn stage_pdf(source: &Path, data_dir: &Path) -> io::Result<StagedPdf> {
    let mut input = File::open(source)?;

    if !input.metadata()?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "请选择一个普通文件",
        ));
    }

    let original_name = source
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "无法读取文件名"))?
        .to_string_lossy()
        .into_owned();

    // 最小格式检查，不代表 PDF 内容完整或一定能够阅读。
    let mut header = [0u8; 5];
    input.read_exact(&mut header)?;

    if &header != b"%PDF-" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "文件缺少标准 PDF 文件头",
        ));
    }

    input.seek(SeekFrom::Start(0))?;

    let staging_dir = data_dir.join("staging");
    fs::create_dir_all(&staging_dir)?;

    let document_id = Uuid::new_v4().to_string();
    let staged_path = staging_dir.join(format!("{document_id}.pdf"));

    // 仅创建新文件，避免覆盖已有内容。
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&staged_path)?;

    let copy_result = (|| -> io::Result<String> {
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 64 * 1024];

        loop {
            let count = input.read(&mut buffer)?;

            if count == 0 {
                break;
            }

            output.write_all(&buffer[..count])?;
            hasher.update(&buffer[..count]);
        }

        output.sync_all()?;

        let digest = hasher.finalize();

        let hex = digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<Vec<_>>()
            .join("");

        Ok(hex)
    })();

    // 先关闭文件，方便在 Windows 等系统上清理失败产物。
    drop(output);

    match copy_result {
        Ok(sha256) => Ok(StagedPdf {
            document_id,
            original_name,
            staged_path,
            sha256,
        }),
        Err(error) => {
            if let Err(cleanup_error) = fs::remove_file(&staged_path) {
                eprintln!("清理失败的临时 PDF 时出错：{cleanup_error}");
            }

            Err(error)
        }
    }
}
