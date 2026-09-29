import { useEffect, useRef, useState } from "react";
import "../shared/styles/global.css";
import { open } from "@tauri-apps/plugin-dialog";
import { enrichPaperMetadata, importPdf, listPapers } from "../shared/ipc/library";
import { ReaderWorkspace } from "../features/reader";
import { MetadataEditor, PaperFolder, TodoDashboard } from "../features/library";
import { MarkdownWorkspace } from "../features/conversion";
import { PaperTreeWorkspace } from "../features/paper-tree";
import { RagWorkspace } from "../features/rag";
import type { Paper, PaperGroup } from "../shared/contracts/library";
import { assignPaperGroup, createPaperGroup, deletePaperGroup, listPaperGroups, renamePaperGroup } from "../shared/ipc/organization";

export default function App() {
  const [papers, setPapers] = useState<Paper[]>([]);
  const [groups, setGroups] = useState<PaperGroup[]>([]);
  const [activeGroup, setActiveGroup] = useState<string>("all");
  const [newGroupName, setNewGroupName] = useState("");
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
  const [paperTreePaper, setPaperTreePaper] = useState<Paper | null>(null);
  const [ragPaper, setRagPaper] = useState<Paper | null>(null);
  const busy = useRef(false);

  useEffect(() => {
    let active = true;
    Promise.all([listPapers(), listPaperGroups()])
      .then(([items, paperGroups]) => { if (active) { setPapers(items); setGroups(paperGroups); } })
      .catch((error) => { if (active) setImportError(`读取文献失败：${String(error)}`); })
      .finally(() => { if (active) setIsLoading(false); });
    return () => { active = false; };
  }, []);

  async function refreshLibrary() {
    const [items, paperGroups] = await Promise.all([listPapers(), listPaperGroups()]);
    setPapers(items); setGroups(paperGroups);
  }

  async function handleCreateGroup() {
    const name = newGroupName.trim();
    if (!name) return;
    try {
      const created = await createPaperGroup(name);
      setGroups((current) => [...current, created]);
      setActiveGroup(created.id); setSelectedId(null); setNewGroupName(""); setMessage("分组已创建。");
    } catch (cause) { setImportError(String(cause)); }
  }

  async function handleRenameGroup(group: PaperGroup) {
    const name = window.prompt("新的分组名称", group.name)?.trim();
    if (!name || name === group.name) return;
    try { await renamePaperGroup(group.id, name); await refreshLibrary(); }
    catch (cause) { setImportError(String(cause)); }
  }

  async function handleDeleteGroup(group: PaperGroup) {
    if (!window.confirm(`删除分组“${group.name}”？其中的论文会移回未归类。`)) return;
    try {
      await deletePaperGroup(group.id); await refreshLibrary();
      if (activeGroup === group.id) setActiveGroup("ungrouped");
      setSelectedId(null);
    } catch (cause) { setImportError(String(cause)); }
  }

  async function handleAssignGroup(paper: Paper, groupId: string | null) {
    try {
      await assignPaperGroup(paper.id, groupId); await refreshLibrary();
      setMessage(groupId ? "论文已移入分组。" : "论文已移至未归类。");
    } catch (cause) { setImportError(String(cause)); }
  }

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
        if (!result.duplicate) setActiveGroup("ungrouped");
        setSelectedId(result.paperId);
        setExpandedIds((current) => new Set(current).add(result.paperId));
        setMessage(result.duplicate ? "此 PDF 已在文献库中，已选中原记录。" : "PDF 已导入，正在自动识别论文信息…");
        try {
          const refreshed = await listPapers();
          setPapers(refreshed);
          const imported = refreshed.find((paper) => paper.id === result.paperId);
          if (imported) setReading({ id: imported.id, title: imported.title });
          if (!result.duplicate) {
            enrichPaperMetadata(result.paperId).then((enriched) => {
              setPapers((current) => current.map((paper) => paper.id === enriched.id ? enriched : paper));
              setReading((current) => current?.id === enriched.id ? { id: enriched.id, title: enriched.title } : current);
              setMessage("PDF 已导入，自动识别已完成；可在文献详情中核对结果。");
            }).catch((error) => {
              setMessage(`PDF 已导入；自动识别暂未完成，可稍后手动编辑：${String(error)}`);
            });
          }
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


  const groupPapers = papers.filter((paper) => activeGroup === "all"
    || (activeGroup === "ungrouped" ? paper.groupId === null : paper.groupId === activeGroup));
  const filteredPapers = groupPapers.filter((paper) =>
    paper.title.toLocaleLowerCase().includes(query.trim().toLocaleLowerCase()),
  );
  const selectedPaper = papers.find((paper) => paper.id === selectedId);
  const activeGroupName = activeGroup === "all" ? "全部文献"
    : activeGroup === "ungrouped" ? "未归类"
      : groups.find((group) => group.id === activeGroup)?.name ?? "文献分组";

  if (reading) {
    return <ReaderWorkspace key={reading.id} paperId={reading.id} title={reading.title}
      onClose={() => setReading(null)} />;
  }

  if (convertingPaper) {
    return <MarkdownWorkspace key={convertingPaper.id} paper={convertingPaper}
      onConverted={() => {
        setPapers((current) => current.map((paper) => paper.id === convertingPaper.id
          ? { ...paper, hasMarkdown: true, hasTranslation: false, hasRag: false }
          : paper));
      }}
      onTranslated={() => {
        setPapers((current) => current.map((paper) => paper.id === convertingPaper.id
          ? { ...paper, hasTranslation: true }
          : paper));
      }}
      onClose={() => setConvertingPaper(null)} />;
  }

  if (paperTreePaper) {
    return <PaperTreeWorkspace key={paperTreePaper.id} paper={paperTreePaper}
      onSaved={() => {
        setPapers((current) => current.map((paper) => paper.id === paperTreePaper.id
          ? { ...paper, hasPaperTree: true }
          : paper));
        setPaperTreePaper((current) => current ? { ...current, hasPaperTree: true } : current);
      }}
      onClose={() => setPaperTreePaper(null)} />;
  }

  if (ragPaper) {
    return <RagWorkspace key={ragPaper.id} paper={ragPaper} papers={papers}
      onIndexed={() => {
        setPapers((current) => current.map((paper) => paper.id === ragPaper.id
          ? { ...paper, hasRag: true } : paper));
        setRagPaper((current) => current ? { ...current, hasRag: true } : current);
      }}
      onClose={() => setRagPaper(null)} />;
  }

  return (
    <div className="workspace">
      <aside className="sidebar group-sidebar">
        <h1 className="brand">Reflo</h1>
        <nav className="group-navigation" aria-label="文献分组">
          <button className="nav-item" aria-current={activeGroup === "all" ? "page" : undefined}
            onClick={() => { setActiveGroup("all"); setSelectedId(null); }}>
            <span><b aria-hidden="true">▤</b> 全部文献</span><em>{papers.length}</em>
          </button>
          <button className="nav-item" aria-current={activeGroup === "ungrouped" ? "page" : undefined}
            onClick={() => { setActiveGroup("ungrouped"); setSelectedId(null); }}>
            <span><b aria-hidden="true">◇</b> 未归类</span><em>{papers.filter((paper) => paper.groupId === null).length}</em>
          </button>
          <div className="group-section-heading"><span>我的分组</span><span>{groups.length}</span></div>
          {groups.map((group) => <div className="group-nav-row" key={group.id}>
            <button className="nav-item" aria-current={activeGroup === group.id ? "page" : undefined}
              onClick={() => { setActiveGroup(group.id); setSelectedId(null); }}>
              <span><b aria-hidden="true">□</b> {group.name}</span><em>{group.paperCount}</em>
            </button>
            <button className="group-menu-button" title="重命名分组" onClick={() => void handleRenameGroup(group)}>···</button>
            <button className="group-menu-button danger" title="删除分组" onClick={() => void handleDeleteGroup(group)}>×</button>
          </div>)}
        </nav>
        <form className="new-group-form" onSubmit={(event) => { event.preventDefault(); void handleCreateGroup(); }}>
          <input maxLength={100} value={newGroupName} placeholder="新建分组"
            onChange={(event) => setNewGroupName(event.target.value)} />
          <button aria-label="创建分组" disabled={!newGroupName.trim()}>＋</button>
        </form>
      </aside>

      <aside className="library paper-sidebar" aria-label="文献列表">
        <header className="library-header">
          <div><h2>{activeGroupName}</h2><p className="muted">{groupPapers.length} 篇文献</p></div>
          <button className="import-button" disabled={isSelecting || isLoading || editing} onClick={handleSelectPdf}>
            {isSelecting ? "处理中…" : "导入"}
          </button>
        </header>
        <div className="library-search">
          <input aria-label="搜索标题" type="search" placeholder="搜索文献…" value={query}
            onChange={(event) => setQuery(event.target.value)} />
        </div>
        {message && <p className="library-message" role="status">{message}</p>}
        {importError && <p className="library-error" role="alert">{importError}</p>}
        {isLoading && <p className="muted paper-list-notice">正在读取文献…</p>}
        {!isLoading && filteredPapers.length === 0 && (
          <div className="paper-list-empty"><span aria-hidden="true">▱</span><p>{query.trim() ? "没有匹配的论文" : "这个分组还没有论文"}</p></div>
        )}
        <ul className="paper-list">
          {filteredPapers.map((paper) => <PaperFolder key={paper.id} paper={paper}
            expanded={expandedIds.has(paper.id)} selected={selectedId === paper.id} disabled={editing || isSelecting}
            onSelect={() => setSelectedId(paper.id)} onToggle={() => {
              setSelectedId(paper.id); setExpandedIds((current) => {
                const next = new Set(current); if (next.has(paper.id)) next.delete(paper.id); else next.add(paper.id); return next;
              });
            }}
            onOpenPdf={() => { setSelectedId(paper.id); setReading({ id: paper.id, title: paper.title }); }}
            onOpenMarkdown={() => { setSelectedId(paper.id); setConvertingPaper(paper); }}
            onOpenTranslation={() => { setSelectedId(paper.id); setConvertingPaper(paper); }}
            onOpenPaperTree={() => { setSelectedId(paper.id); setPaperTreePaper(paper); }}
            onOpenRag={() => { setSelectedId(paper.id); setRagPaper(paper); }} />)}
        </ul>
      </aside>

      <main className="main-content">
        {selectedPaper ? <section className="paper-details" aria-label="文献详情">
          <header className="paper-details-header">
            <div><span className="dashboard-eyebrow">PAPER</span><h2>{selectedPaper.title}</h2>
              <p>{selectedPaper.authors.join("、") || "作者待识别"}</p></div>
            <button className="secondary-button" onClick={() => setSelectedId(null)}>关闭</button>
          </header>
          <div className="paper-detail-body">
            <div className="paper-tools">
              <button className="import-button open-pdf" disabled={editing || isSelecting}
                onClick={() => setReading({ id: selectedPaper.id, title: selectedPaper.title })}>阅读 PDF</button>
              <button className="secondary-button" disabled={editing || isSelecting}
                onClick={() => setConvertingPaper(selectedPaper)}>转为 Markdown</button>
              <button className="secondary-button" disabled={editing || isSelecting}
                onClick={() => setPaperTreePaper(selectedPaper)}>Paper Tree</button>
              <button className="secondary-button" disabled={editing || isSelecting}
                onClick={() => setRagPaper(selectedPaper)}>论文问答</button>
            </div>
            <label className="paper-group-select">所属分组
              <select value={selectedPaper.groupId ?? ""} disabled={editing}
                onChange={(event) => void handleAssignGroup(selectedPaper, event.target.value || null)}>
                <option value="">未归类</option>
                {groups.map((group) => <option key={group.id} value={group.id}>{group.name}</option>)}
              </select>
            </label>
            <MetadataEditor key={`${selectedPaper.id}:${selectedPaper.revision}`} paper={selectedPaper}
              disabled={isSelecting} onEditingChange={setEditing} onSaved={(updated) => {
                setPapers((current) => current.map((paper) => paper.id === updated.id ? updated : paper));
                setMessage("文献信息已保存。");
              }} />
          </div>
        </section> : <TodoDashboard />}
      </main>
    </div>
  );
}
