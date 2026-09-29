import type { Paper } from "../../shared/contracts/library";

type Props = {
  paper: Paper;
  expanded: boolean;
  selected: boolean;
  disabled: boolean;
  onSelect: () => void;
  onToggle: () => void;
  onOpenPdf: () => void;
  onOpenMarkdown: () => void;
  onOpenTranslation: () => void;
  onOpenPaperTree: () => void;
  onOpenRag: () => void;
};

export function PaperFolder({
  paper, expanded, selected, disabled, onSelect, onToggle, onOpenPdf, onOpenMarkdown, onOpenTranslation,
  onOpenPaperTree,
  onOpenRag,
}: Props) {
  return (
    <li className="paper-folder">
      <div className="paper-folder-row" data-selected={selected || undefined}>
        <button className="folder-disclosure" aria-label={expanded ? "收起论文文件" : "展开论文文件"}
          aria-expanded={expanded} disabled={disabled} onClick={onToggle}>
          <span aria-hidden="true">{expanded ? "⌄" : "›"}</span>
        </button>
        <button className="paper-folder-summary" aria-pressed={selected} disabled={disabled} onClick={onSelect}>
          <span className="folder-icon" aria-hidden="true">{expanded ? "▾" : "▸"}</span>
          <span className="paper-folder-copy">
            <span className="paper-title">{paper.title}</span>
            <span className="muted">{paper.authors.join("、") || "作者待填写"} · {paper.year ?? "年份待填写"}</span>
          </span>
        </button>
      </div>

      {expanded && (
        <ul className="paper-artifact-list" aria-label={`${paper.title} 的文件`}>
          <li>
            <button className="paper-artifact" disabled={disabled} onClick={onOpenPdf}>
              <span className="artifact-icon pdf" aria-hidden="true">PDF</span>
              <span><strong>PDF 原文</strong><small>{paper.originalName}</small></span>
              <span className="artifact-status ready">可阅读</span>
            </button>
          </li>
          <li>
            <button className="paper-artifact" disabled={disabled} onClick={onOpenMarkdown}>
              <span className="artifact-icon markdown" aria-hidden="true">M↓</span>
              <span><strong>Markdown</strong><small>{paper.hasMarkdown ? "MinerU 转换结果" : "尚未生成，点击开始转换"}</small></span>
              <span className={`artifact-status ${paper.hasMarkdown ? "ready" : "pending"}`}>{paper.hasMarkdown ? "已生成" : "待转换"}</span>
            </button>
          </li>
          <li>
            <button className="paper-artifact" disabled={disabled} onClick={onOpenPaperTree}>
              <span className="artifact-icon tree" aria-hidden="true">树</span>
              <span><strong>Paper Tree</strong><small>{paper.hasPaperTree ? "论文结构树" : "使用模板梳理论文结构"}</small></span>
              <span className={`artifact-status ${paper.hasPaperTree ? "ready" : "pending"}`}>{paper.hasPaperTree ? "已保存" : "待填写"}</span>
            </button>
          </li>
          <li>
            <button className="paper-artifact" disabled={disabled} onClick={onOpenRag}>
              <span className="artifact-icon rag" aria-hidden="true">问</span>
              <span><strong>论文问答</strong><small>{paper.hasRag ? "基于当前论文检索回答" : "为论文创建独立 RAG"}</small></span>
              <span className={`artifact-status ${paper.hasRag ? "ready" : "pending"}`}>{paper.hasRag ? "已索引" : "待索引"}</span>
            </button>
          </li>
          <li>
            <button className="paper-artifact" disabled={disabled} onClick={onOpenTranslation}>
              <span className="artifact-icon translation" aria-hidden="true">译</span>
              <span><strong>译文</strong><small>{paper.hasTranslation ? "Markdown 全文翻译结果" : "在 Markdown 工作区一键生成"}</small></span>
              <span className={`artifact-status ${paper.hasTranslation ? "ready" : "pending"}`}>{paper.hasTranslation ? "已生成" : "待翻译"}</span>
            </button>
          </li>
        </ul>
      )}
    </li>
  );
}
