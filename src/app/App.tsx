import { useEffect, useRef, useState } from "react";
import "../shared/styles/global.css";
import { open } from "@tauri-apps/plugin-dialog";
import { importPdf, listPapers } from "../shared/ipc/library";
import { ReaderWorkspace } from "../features/reader";
import { MetadataEditor, PaperFolder } from "../features/library";
import { MarkdownWorkspace } from "../features/conversion";
import type { Paper } from "../shared/contracts/library";

export default function App() {
  const [papers, setPapers] = useState<Paper[]>([]);
  const [message, setMessage] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isSelecting, setIsSelecting] = useState(false);
  const [importError, setImportError] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [expandedIds, setExpandedIds] = useState<Set<string>>(() => new Set());
  const [query, setQuery] = useState("");
  const [editing, setEditing] = useState(false);
  const [reading, setReading] = useState<{ id: string; title: string } | null>(null);
  const [convertingPaper, setConvertingPaper] = useState<Paper | null>(null);
  const busy = useRef(false);

  useEffect(() => {
    let active = true;
    listPapers()
      .then((items) => { if (active) setPapers(items); })
      .catch((error) => { if (active) setImportError(`读取文献失败：${String(error)}`); })
      .finally(() => { if (active) setIsLoading(false); });
    return () => { active = false; };
  }, []);

  async function handleSelectPdf() {
    if (busy.current || editing) return;
    busy.current = true;
    setIsSelecting(true);
    setMessage(null);
    setImportError(null);

    try {
      const path = await open({
        title: "选择论文 PDF",
        multiple: false,
        directory: false,
        filters: [
          {
            name: "PDF 文档",
            extensions: ["pdf"],
          },
        ],
      });

      if (path !== null) {
        const result = await importPdf(path);
        setQuery("");
        setSelectedId(result.paperId);
        setExpandedIds((current) => new Set(current).add(result.paperId));
        setMessage(result.duplicate ? "此 PDF 已在文献库中，已选中原记录。" : "PDF 已导入并保存到本地文献库。");
        try {
          const refreshed = await listPapers();
          setPapers(refreshed);
          const imported = refreshed.find((paper) => paper.id === result.paperId);
          if (imported) setReading({ id: imported.id, title: imported.title });
        } catch (error) {
          setImportError(`文件已处理，但列表刷新失败，请重启应用：${String(error)}`);
        }
      }
    } catch (error) {
      setImportError(`导入失败：${String(error)}`);
    } finally {
      busy.current = false;
      setIsSelecting(false);
    }
  }


  const filteredPapers = papers.filter((paper) =>
    paper.title.toLocaleLowerCase().includes(query.trim().toLocaleLowerCase()),
  );
  const selectedPaper = papers.find((paper) => paper.id === selectedId);

  if (reading) {
    return <ReaderWorkspace key={reading.id} paperId={reading.id} title={reading.title}
      onClose={() => setReading(null)} />;
  }

  if (convertingPaper) {
    return <MarkdownWorkspace key={convertingPaper.id} paper={convertingPaper}
      onConverted={() => {
        setPapers((current) => current.map((paper) => paper.id === convertingPaper.id
          ? { ...paper, hasMarkdown: true }
          : paper));
      }}
      onClose={() => setConvertingPaper(null)} />;
  }

  return (
    <div className="workspace">
      <aside className="sidebar">
        <h1 className="brand">Reflo</h1>

        <nav aria-label="文献导航">
          <button className="nav-item" aria-current="page">
            全部文献
            <span>{papers.length}</span>
          </button>
        </nav>
      </aside>

      <main className="library">
        <header className="library-header">
          <div>
            <h2>全部文献</h2>
            <p className="muted">{papers.length} 篇文献 · 本地文献库</p>
          </div>

          <button
            className="import-button"
            disabled={isSelecting || isLoading || editing}
            onClick={handleSelectPdf}
          >
            {isSelecting ? "处理中…" : "导入 PDF"}
          </button>
        </header>

        <div className="library-search">
          <label htmlFor="paper-search">搜索标题</label>
          <input id="paper-search" type="search" placeholder="输入论文标题…"
            value={query} onChange={(event) => setQuery(event.target.value)} />
          {query.trim() && <span className="muted" role="status">找到 {filteredPapers.length} 篇文献</span>}
        </div>

        {message && (
        <p
          className="muted"
          role="status"
          style={{ padding: "12px 24px", overflowWrap: "anywhere" }}
        >
          {message}
        </p>
        )}

      {importError && (
        <p role="alert" style={{ padding: "12px 24px", color: "#b91c1c" }}>
          {importError}
        </p>
      )}

        {isLoading && <p className="muted" role="status" style={{ padding: 24 }}>正在读取文献…</p>}
        {!isLoading && papers.length === 0 && !importError && (
          <p className="muted" style={{ padding: 24 }}>文献库为空，点击“导入 PDF”添加第一篇论文。</p>
        )}
        {!isLoading && papers.length > 0 && filteredPapers.length === 0 && (
          <p className="muted" style={{ padding: 24 }}>没有匹配的标题，请尝试其他关键词。</p>
        )}
        <ul className="paper-list" aria-label="文献列表">
          {filteredPapers.map((paper) => (
            <PaperFolder key={paper.id} paper={paper} expanded={expandedIds.has(paper.id)}
              selected={selectedId === paper.id} disabled={editing || isSelecting}
              onSelect={() => setSelectedId(paper.id)}
              onToggle={() => {
                setSelectedId(paper.id);
                setExpandedIds((current) => {
                  const next = new Set(current);
                  if (next.has(paper.id)) next.delete(paper.id); else next.add(paper.id);
                  return next;
                });
              }}
              onOpenPdf={() => {
                setSelectedId(paper.id);
                setReading({ id: paper.id, title: paper.title });
              }}
              onOpenMarkdown={() => {
                setSelectedId(paper.id);
                setConvertingPaper(paper);
              }} />
          ))}
        </ul>
      </main>

      <aside className="details" aria-label="文献详情">
        <h2>文献详情</h2>

        {selectedPaper && (
          <div className="paper-tools">
            <button className="import-button open-pdf" disabled={editing || isSelecting}
              onClick={() => setReading({ id: selectedPaper.id, title: selectedPaper.title })}>阅读 PDF</button>
            <button className="secondary-button" disabled={editing || isSelecting}
              onClick={() => setConvertingPaper(selectedPaper)}>转为 Markdown</button>
          </div>
        )}
        {selectedPaper ? (
          <MetadataEditor
            key={`${selectedPaper.id}:${selectedPaper.revision}`}
            paper={selectedPaper}
            disabled={isSelecting}
            onEditingChange={setEditing}
            onSaved={(updated) => {
              setPapers((current) => current.map((paper) => paper.id === updated.id ? updated : paper));
              setMessage("文献信息已保存。");
            }}
          />
        ) : (
          <p className="muted">选择一篇文献，查看详细信息。</p>
        )}
      </aside>
    </div>
  );
}
