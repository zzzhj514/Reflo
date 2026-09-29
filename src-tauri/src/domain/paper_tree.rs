use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperTreeNode {
    pub id: String,
    pub title: String,
    pub note: String,
    pub children: Vec<PaperTreeNode>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperTree {
    pub paper_id: String,
    pub title: String,
    pub nodes: Vec<PaperTreeNode>,
    pub updated_at: Option<String>,
    pub saved: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavePaperTree {
    pub paper_id: String,
    pub title: String,
    pub nodes: Vec<PaperTreeNode>,
}
