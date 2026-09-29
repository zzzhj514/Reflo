use std::{collections::HashSet, fs, path::Path};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    application::{paper_tree_prompts, translation},
    domain::paper_tree::{PaperTree, PaperTreeNode, SavePaperTree},
    infrastructure::db::{conversions, paper_trees, papers, reader},
};

const DIRECT_SOURCE_CHARS: usize = 40_000;
const SOURCE_CHUNK_CHARS: usize = 24_000;
const MAX_SOURCE_CHUNKS: usize = 6;

#[derive(Deserialize)]
struct GeneratedTree {
    title: String,
    nodes: Vec<GeneratedNode>,
}

#[derive(Deserialize)]
struct GeneratedNode {
    title: String,
    #[serde(default)]
    note: String,
    #[serde(default)]
    children: Vec<GeneratedNode>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperTreeGeneration {
    pub tree: PaperTree,
    pub provider: String,
    pub model: String,
    pub source_truncated: bool,
}

fn node(title: &str, note: &str, children: Vec<PaperTreeNode>) -> PaperTreeNode {
    PaperTreeNode {
        id: Uuid::new_v4().to_string(),
        title: title.into(),
        note: note.into(),
        children,
    }
}

fn section(title: &str, children: Vec<PaperTreeNode>) -> PaperTreeNode {
    node(title, "", children)
}

fn default_nodes() -> Vec<PaperTreeNode> {
    vec![
        section(
            "Abstract",
            vec![
                node(
                    "Technical challenge",
                    "现有方法面对的核心技术问题是什么？",
                    vec![],
                ),
                section(
                    "Key insight / motivation",
                    vec![
                        node("Insight", "这篇论文观察到了什么？", vec![]),
                        node("Benefit", "该洞察为什么有帮助？", vec![]),
                    ],
                ),
                section(
                    "Technical contributions",
                    vec![
                        node("Contribution 1", "方法与技术优势", vec![]),
                        node("Contribution 2", "方法与技术优势", vec![]),
                    ],
                ),
                node("Experiment summary", "主要结果与结论", vec![]),
            ],
        ),
        section(
            "Introduction",
            vec![
                node(
                    "Task and application",
                    "任务定义、输入输出与应用场景",
                    vec![],
                ),
                section(
                    "Technical challenges",
                    vec![
                        node("Challenge 1", "已有方法、局限与技术原因", vec![]),
                        node("Challenge 2", "已有方法、局限与技术原因", vec![]),
                    ],
                ),
                section(
                    "Proposed pipeline",
                    vec![
                        node("Key insight", "整体思路", vec![]),
                        node("Contribution 1", "解决的问题、做法与优势", vec![]),
                        node("Contribution 2", "解决的问题、做法与优势", vec![]),
                    ],
                ),
                node("Demos / applications", "演示与应用", vec![]),
            ],
        ),
        section(
            "Method",
            vec![
                section(
                    "Overview",
                    vec![
                        node("Task", "输入与输出", vec![]),
                        node("Pipeline", "Step 1 → Step 2 → Step 3", vec![]),
                    ],
                ),
                node(
                    "Pipeline module 1",
                    "Motivation、做法、为什么有效、技术优势",
                    vec![],
                ),
                node(
                    "Pipeline module 2",
                    "Motivation、做法、为什么有效、技术优势",
                    vec![],
                ),
            ],
        ),
        section(
            "Experiments",
            vec![
                node(
                    "Comparison experiments",
                    "数据集、指标、基线和主要结果",
                    vec![],
                ),
                section(
                    "Ablation studies",
                    vec![
                        node("Component impact", "各贡献与组件的影响", vec![]),
                        node("Design choices", "关键设计选择的影响", vec![]),
                    ],
                ),
            ],
        ),
        section(
            "Limitation",
            vec![node(
                "Limitations and future work",
                "局限、失败案例与改进方向",
                vec![],
            )],
        ),
    ]
}

pub fn get(data_dir: &Path, paper_id: &str, paper_title: &str) -> Result<PaperTree, String> {
    let database = data_dir.join("reflo.sqlite");
    if !paper_trees::paper_exists(&database, paper_id).map_err(|e| e.to_string())? {
        return Err("文献不存在".into());
    }
    if let Some(tree) = paper_trees::get(&database, paper_id).map_err(|e| e.to_string())? {
        return Ok(tree);
    }
    Ok(PaperTree {
        paper_id: paper_id.into(),
        title: paper_title.trim().to_owned(),
        nodes: default_nodes(),
        updated_at: None,
        saved: false,
    })
}

pub fn save(data_dir: &Path, mut input: SavePaperTree) -> Result<PaperTree, String> {
    let database = data_dir.join("reflo.sqlite");
    if !paper_trees::paper_exists(&database, &input.paper_id).map_err(|e| e.to_string())? {
        return Err("文献不存在".into());
    }
    input.title = input.title.trim().to_owned();
    if input.title.is_empty() || input.title.chars().count() > 2000 {
        return Err("Paper Tree 标题须为 1 到 2000 个字符".into());
    }
    if input.nodes.is_empty() {
        return Err("Paper Tree 至少需要一个节点".into());
    }
    let mut ids = HashSet::new();
    let mut count = 0usize;
    validate_nodes(&mut input.nodes, 1, &mut count, &mut ids)?;
    paper_trees::save(&database, &input.paper_id, &input.title, &input.nodes)
        .map_err(|e| e.to_string())
}

pub fn generate(data_dir: &Path, paper_id: &str) -> Result<PaperTreeGeneration, String> {
    let database = data_dir.join("reflo.sqlite");
    let paper = papers::list_papers(&database)
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|paper| paper.id == paper_id)
        .ok_or("文献不存在")?;
    let document = reader::document_for_paper(&database, paper_id)
        .map_err(|error| error.to_string())?
        .ok_or("这篇文献没有 PDF 文件")?;
    let markdown_record = conversions::find_markdown(&database, &document.id)
        .map_err(|error| error.to_string())?
        .ok_or("请先将 PDF 转换为 Markdown，再生成 Paper Tree")?;
    let library = data_dir
        .join("library")
        .canonicalize()
        .map_err(|_| "找不到本地文献库")?;
    let markdown_path = data_dir
        .join(markdown_record.relative_path)
        .canonicalize()
        .map_err(|_| "Markdown 文件已丢失")?;
    if !markdown_path.starts_with(&library) || !markdown_path.is_file() {
        return Err("Markdown 文件不在本地文献库范围内".into());
    }
    let markdown = fs::read_to_string(markdown_path).map_err(|error| error.to_string())?;
    if markdown.trim().is_empty() {
        return Err("Markdown 内容为空，无法生成 Paper Tree".into());
    }

    let markdown_chars = markdown.chars().count();
    let source_truncated = markdown_chars > SOURCE_CHUNK_CHARS * MAX_SOURCE_CHUNKS;
    let completion = if markdown_chars <= DIRECT_SOURCE_CHARS {
        translation::complete_model(
            data_dir,
            paper_tree_prompts::FINAL_TREE_SYSTEM,
            &paper_tree_prompts::direct_user_prompt(&paper.title, &markdown),
        )?
    } else {
        let chunks = source_chunks(&markdown);
        let mut analyses = Vec::with_capacity(chunks.len());
        let mut last_provider = String::new();
        let mut last_model = String::new();
        for (index, chunk) in chunks.iter().enumerate() {
            let result = translation::complete_model(
                data_dir,
                paper_tree_prompts::CHUNK_ANALYSIS_SYSTEM,
                &paper_tree_prompts::chunk_user_prompt(chunk, index + 1, chunks.len()),
            )?;
            last_provider = result.provider;
            last_model = result.model;
            analyses.push(result.content);
        }
        let mut result = translation::complete_model(
            data_dir,
            paper_tree_prompts::FINAL_TREE_SYSTEM,
            &paper_tree_prompts::synthesis_user_prompt(&paper.title, &analyses),
        )?;
        if result.provider.is_empty() {
            result.provider = last_provider;
            result.model = last_model;
        }
        result
    };

    let generated = parse_generated_tree(&completion.content)?;
    let title = if generated.title.trim().is_empty() {
        paper.title
    } else {
        generated.title
    };
    let nodes = generated.nodes.into_iter().map(with_id).collect();
    let tree = save(
        data_dir,
        SavePaperTree {
            paper_id: paper_id.to_owned(),
            title,
            nodes,
        },
    )?;
    Ok(PaperTreeGeneration {
        tree,
        provider: completion.provider,
        model: completion.model,
        source_truncated,
    })
}

fn source_chunks(source: &str) -> Vec<String> {
    let characters: Vec<char> = source.chars().collect();
    let available_chars = SOURCE_CHUNK_CHARS * MAX_SOURCE_CHUNKS;
    if characters.len() > available_chars {
        let last_start = characters.len() - SOURCE_CHUNK_CHARS;
        return (0..MAX_SOURCE_CHUNKS)
            .map(|index| {
                let start = index * last_start / (MAX_SOURCE_CHUNKS - 1);
                characters[start..start + SOURCE_CHUNK_CHARS]
                    .iter()
                    .collect()
            })
            .collect();
    }
    let mut chunks = Vec::new();
    let mut current = String::new();
    for line in source.split_inclusive('\n') {
        if !current.is_empty()
            && current.chars().count() + line.chars().count() > SOURCE_CHUNK_CHARS
        {
            chunks.push(std::mem::take(&mut current));
        }
        for character in line.chars() {
            current.push(character);
            if current.chars().count() >= SOURCE_CHUNK_CHARS {
                chunks.push(std::mem::take(&mut current));
            }
        }
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

fn parse_generated_tree(content: &str) -> Result<GeneratedTree, String> {
    let trimmed = content.trim();
    let start = trimmed.find('{').ok_or("模型返回内容中没有 JSON 对象")?;
    let end = trimmed.rfind('}').ok_or("模型返回的 JSON 不完整")?;
    let generated: GeneratedTree = serde_json::from_str(&trimmed[start..=end])
        .map_err(|error| format!("模型返回的 Paper Tree JSON 无效：{error}"))?;
    let required = [
        "Abstract",
        "Introduction",
        "Method",
        "Experiments",
        "Limitation",
    ];
    if generated.nodes.len() != required.len()
        || generated
            .nodes
            .iter()
            .zip(required)
            .any(|(node, title)| node.title.trim() != title)
    {
        return Err("模型返回的 Paper Tree 必须依次包含 Abstract、Introduction、Method、Experiments、Limitation".into());
    }
    Ok(generated)
}

fn with_id(node: GeneratedNode) -> PaperTreeNode {
    PaperTreeNode {
        id: Uuid::new_v4().to_string(),
        title: node.title,
        note: node.note,
        children: node.children.into_iter().map(with_id).collect(),
    }
}

fn validate_nodes(
    nodes: &mut [PaperTreeNode],
    depth: usize,
    count: &mut usize,
    ids: &mut HashSet<String>,
) -> Result<(), String> {
    if depth > 12 {
        return Err("Paper Tree 最多支持 12 层".into());
    }
    for node in nodes {
        *count += 1;
        if *count > 500 {
            return Err("Paper Tree 最多支持 500 个节点".into());
        }
        node.id = node.id.trim().to_owned();
        node.title = node.title.trim().to_owned();
        node.note = node.note.trim().to_owned();
        if node.id.is_empty() || !ids.insert(node.id.clone()) {
            return Err("Paper Tree 节点 ID 无效或重复".into());
        }
        if node.title.is_empty() || node.title.chars().count() > 1000 {
            return Err("节点标题须为 1 到 1000 个字符".into());
        }
        if node.note.chars().count() > 10_000 {
            return Err("单个节点说明不能超过 10000 个字符".into());
        }
        validate_nodes(&mut node.children, depth + 1, count, ids)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::db;

    struct Fixture(std::path::PathBuf);

    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!("reflo-tree-{}", Uuid::new_v4()));
            std::fs::create_dir(&root).unwrap();
            let database = root.join("reflo.sqlite");
            db::initialize(&database).unwrap();
            rusqlite::Connection::open(database)
                .unwrap()
                .execute(
                    "INSERT INTO papers(id, title, authors_json, created_at) VALUES ('paper-1', 'Test Paper', '[]', CURRENT_TIMESTAMP)",
                    [],
                )
                .unwrap();
            Self(root)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn returns_template_then_persists_an_edited_tree() {
        let fixture = Fixture::new();
        let template = get(&fixture.0, "paper-1", "Test Paper").unwrap();
        assert!(!template.saved);
        assert_eq!(template.nodes[0].title, "Abstract");

        let mut nodes = template.nodes;
        nodes[0].note = "Edited".into();
        let saved = save(
            &fixture.0,
            SavePaperTree {
                paper_id: "paper-1".into(),
                title: "My Paper Tree".into(),
                nodes,
            },
        )
        .unwrap();
        assert!(saved.saved);
        assert_eq!(saved.nodes[0].note, "Edited");

        let reopened = get(&fixture.0, "paper-1", "Ignored title").unwrap();
        assert_eq!(reopened.title, "My Paper Tree");
        assert!(reopened.updated_at.is_some());
    }

    #[test]
    fn parses_fenced_model_json_and_rejects_wrong_sections() {
        let json = r#"```json
        {"title":"A Paper","nodes":[
          {"title":"Abstract","note":"a","children":[]},
          {"title":"Introduction","note":"b","children":[]},
          {"title":"Method","note":"c","children":[]},
          {"title":"Experiments","note":"d","children":[]},
          {"title":"Limitation","note":"e","children":[]}
        ]}
        ```"#;
        let parsed = parse_generated_tree(json).unwrap();
        assert_eq!(parsed.nodes.len(), 5);
        assert_eq!(
            with_id(parsed.nodes.into_iter().next().unwrap()).title,
            "Abstract"
        );
        assert!(parse_generated_tree(r#"{"title":"x","nodes":[]}"#).is_err());
        assert_eq!(source_chunks(&"x".repeat(SOURCE_CHUNK_CHARS + 3)).len(), 2);
        let sampled = source_chunks(&"x".repeat(SOURCE_CHUNK_CHARS * MAX_SOURCE_CHUNKS + 99));
        assert_eq!(sampled.len(), MAX_SOURCE_CHUNKS);
        assert!(sampled
            .iter()
            .all(|chunk| chunk.chars().count() == SOURCE_CHUNK_CHARS));
    }
}
