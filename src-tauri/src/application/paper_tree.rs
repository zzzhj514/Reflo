use std::{collections::HashSet, path::Path};

use uuid::Uuid;

use crate::{
    domain::paper_tree::{PaperTree, PaperTreeNode, SavePaperTree},
    infrastructure::db::paper_trees,
};

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
}
