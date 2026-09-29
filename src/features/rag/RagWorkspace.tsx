import { useEffect, useRef, useState } from "react";
import ReactMarkdown from "react-markdown";
import rehypeKatex from "rehype-katex";
import remarkGfm from "remark-gfm";
import remarkMath from "remark-math";
import "katex/dist/katex.min.css";
import type { Paper } from "../../shared/contracts/library";
import type { RagIndexStatus, RagMessage, RagPreferences, RagProvider } from "../../shared/contracts/rag";
import { askPaperRag, getRagIndexStatus, getRagSettings, indexPaperRag, listRagMessages, saveRagSettings } from "../../shared/ipc/rag";
import { TranslationSettings } from "../reader/components/TranslationSettings";

type Props = { paper: Paper; papers: Paper[]; onIndexed: () => void; onClose: () => void };

const DEFAULTS: RagPreferences = {
  provider: "openai", baseUrl: "https://api.openai.com/v1",
  model: "text-embedding-3-small", chunkChars: 2400, topK: 6,
};

export function RagWorkspace({ paper, papers, onIndexed, onClose }: Props) {
  const [settings, setSettings] = useState<RagPreferences>(DEFAULTS);
  const [apiKey, setApiKey] = useState("");
  const [keyConfigured, setKeyConfigured] = useState(false);
  const [configuredProviders, setConfiguredProviders] = useState<RagProvider[]>([]);
  const [indexStatus, setIndexStatus] = useState<RagIndexStatus | null>(null);
  const [messages, setMessages] = useState<RagMessage[]>([]);
  const [question, setQuestion] = useState("");
  const [loading, setLoading] = useState(true);
  const [indexing, setIndexing] = useState(false);
  const [asking, setAsking] = useState(false);
  const [savingSettings, setSavingSettings] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [showScope, setShowScope] = useState(false);
  const [selectedIds, setSelectedIds] = useState<string[]>([paper.id]);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const endRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    let active = true;
    Promise.all([getRagSettings(), getRagIndexStatus(paper.id)])
      .then(([ragSettings, status]) => {
        if (!active) return;
        setSettings(ragSettings.preferences);
        setKeyConfigured(ragSettings.keyConfigured);
        setConfiguredProviders(ragSettings.configuredProviders);
        setIndexStatus(status);
        if (!ragSettings.keyConfigured) setShowSettings(true);
      })
      .catch((cause) => { if (active) setError(`读取 RAG 数据失败：${String(cause)}`); })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [paper.id]);

  const scopeKey = [...selectedIds].sort().join(":");
  useEffect(() => {
    let active = true;
    setMessages([]);
    listRagMessages(selectedIds).then((history) => { if (active) setMessages(history); })
      .catch((cause) => { if (active) setError(`读取问答历史失败：${String(cause)}`); });
    return () => { active = false; };
  // The sorted key represents the set; array order does not create another conversation.
  }, [scopeKey]);

  useEffect(() => { endRef.current?.scrollIntoView({ behavior: "smooth" }); }, [messages, asking]);

  function chooseProvider(provider: RagProvider) {
    setSettings((current) => {
      if (provider === "openai") return { ...current, provider, baseUrl: "https://api.openai.com/v1", model: "text-embedding-3-small" };
      if (provider === "qwen") return { ...current, provider, baseUrl: "https://dashscope.aliyuncs.com/compatible-mode/v1", model: "qwen3.7-text-embedding-flash" };
      return { ...current, provider };
    });
    setApiKey("");
    setKeyConfigured(configuredProviders.includes(provider));
  }

  async function persistSettings() {
    if (savingSettings || (!keyConfigured && !apiKey.trim())) return;
    setSavingSettings(true); setError(null); setNotice(null);
    try {
      const status = await saveRagSettings(settings, apiKey || undefined);
      setSettings(status.preferences); setKeyConfigured(status.keyConfigured); setApiKey("");
      setConfiguredProviders(status.configuredProviders);
      setNotice("Embedding 设置已保存。模型或分块设置变化后请重新创建索引。");
    } catch (cause) { setError(String(cause)); }
    finally { setSavingSettings(false); }
  }

  async function buildIndex() {
    if (indexing || !paper.hasMarkdown) return;
    setIndexing(true); setError(null); setNotice("正在分块并生成 Embedding，请保持窗口打开…");
    try {
      const status = await indexPaperRag(paper.id);
      setIndexStatus(status); onIndexed(); setNotice(`索引完成：${status.chunkCount} 个片段。`);
    } catch (cause) { setError(`索引失败：${String(cause)}`); setNotice(null); }
    finally { setIndexing(false); }
  }

  async function ask() {
    const value = question.trim();
    if (!value || asking || !scopeReady) return;
    setAsking(true); setError(null); setQuestion("");
    setMessages((current) => [...current, {
      id: `pending-${Date.now()}`, role: "user", content: value, citations: [], createdAt: new Date().toISOString(),
    }]);
    try {
      await askPaperRag(selectedIds, value);
      setMessages(await listRagMessages(selectedIds));
    } catch (cause) {
      setMessages((current) => current.filter((item) => !item.id.startsWith("pending-")));
      setQuestion(value); setError(`问答失败：${String(cause)}`);
    } finally { setAsking(false); }
  }

  function togglePaper(id: string) {
    setSelectedIds((current) => current.includes(id)
      ? (current.length === 1 ? current : current.filter((value) => value !== id))
      : (current.length >= 20 ? current : [...current, id]));
  }

  const selectablePapers = papers.filter((item) => item.hasRag || item.id === paper.id);
  const scopeReady = selectedIds.every((id) => {
    const item = papers.find((candidate) => candidate.id === id);
    return item?.hasRag || (id === paper.id && indexStatus?.indexed);
  });
  const scopeTitle = selectedIds.length === 1
    ? papers.find((item) => item.id === selectedIds[0])?.title ?? paper.title
    : `${selectedIds.length} 篇论文联合问答`;

  return (
    <div className="rag-workspace">
      <header className="rag-heading">
        <button className="secondary-button" disabled={indexing || asking} onClick={onClose}>返回文献库</button>
        <div><h2>论文问答</h2><p>{scopeTitle}</p></div>
        {indexStatus?.indexed && <span className="rag-index-badge">{indexStatus.chunkCount} 个片段</span>}
        <button className="secondary-button" disabled={asking || indexing}
          onClick={() => setShowScope((value) => !value)}>问答范围 · {selectedIds.length}</button>
        <button className="secondary-button" onClick={() => setShowSettings(true)}>RAG 设置</button>
        <button className="primary-button" disabled={indexing || !paper.hasMarkdown || !keyConfigured}
          onClick={() => void buildIndex()}>{indexing ? "索引中…" : indexStatus?.indexed ? "重新索引" : "创建索引"}</button>
      </header>

      {error && <p className="rag-error" role="alert">{error}</p>}
      {notice && <p className="rag-notice" role="status">{notice}</p>}
      {loading ? <p className="reader-notice">正在读取论文问答…</p> : (
        <main className="rag-chat">
          {messages.length === 0 && (
            <section className="rag-empty">
              <span aria-hidden="true">RAG</span>
              <h3>{scopeReady ? `向${selectedIds.length === 1 ? "这篇论文" : "所选论文"}提问` : "先为所选论文创建检索索引"}</h3>
              <p>{scopeReady
                ? `回答只使用所选 ${selectedIds.length} 篇论文的 Markdown 片段，并附上论文和章节来源。`
                : paper.hasMarkdown ? "配置 Embedding API 后点击右上角“创建索引”。" : "请先将 PDF 转换为 Markdown。"}</p>
              {scopeReady && <div className="rag-suggestions">
                {["这篇论文解决了什么问题？", "核心方法和技术贡献是什么？", "实验如何证明方法有效？", "作者提到了哪些局限？"].map((item) =>
                  <button key={item} onClick={() => setQuestion(item)}>{item}</button>)}
              </div>}
            </section>
          )}
          <div className="rag-message-list">
            {messages.map((message) => <article key={message.id} className={`rag-message ${message.role}`}>
              <header>{message.role === "user" ? "你" : "Reflo"}</header>
              {message.role === "assistant" ? <div className="rag-markdown">
                <ReactMarkdown remarkPlugins={[remarkGfm, remarkMath]} rehypePlugins={[rehypeKatex]}>{message.content}</ReactMarkdown>
              </div> : <p>{message.content}</p>}
              {message.citations.length > 0 && <div className="rag-citations">
                {message.citations.map((citation) => <details key={citation.chunkId}>
                  <summary>[{citation.number}] {citation.headingPath}</summary>
                  <blockquote><ReactMarkdown remarkPlugins={[remarkGfm, remarkMath]} rehypePlugins={[rehypeKatex]}>{citation.excerpt}</ReactMarkdown></blockquote>
                </details>)}
              </div>}
            </article>)}
            {asking && <article className="rag-message assistant pending"><header>Reflo</header><p>正在检索论文并组织回答…</p></article>}
            <div ref={endRef} />
          </div>
        </main>
      )}

      <footer className="rag-composer">
        <textarea value={question} disabled={asking || !scopeReady} rows={2}
          placeholder={scopeReady ? `针对所选 ${selectedIds.length} 篇论文提问…（⌘/Ctrl + Enter 发送）` : "请先为所选论文创建索引"}
          onChange={(event) => setQuestion(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) { event.preventDefault(); void ask(); }
          }} />
        <button className="primary-button" disabled={asking || !question.trim() || !scopeReady}
          onClick={() => void ask()}>{asking ? "回答中…" : "发送"}</button>
      </footer>

      {showScope && <aside className="rag-scope-panel">
        <header><div><h3>问答范围</h3><p>勾选 1–20 篇已索引论文</p></div><button aria-label="关闭问答范围" onClick={() => setShowScope(false)}>×</button></header>
        <div className="rag-scope-list">
          {selectablePapers.map((item) => <label key={item.id}>
            <input type="checkbox" checked={selectedIds.includes(item.id)}
              disabled={asking || (!item.hasRag && !(item.id === paper.id && indexStatus?.indexed))}
              onChange={() => togglePaper(item.id)} />
            <span><strong>{item.title}</strong><small>{item.hasRag || (item.id === paper.id && indexStatus?.indexed) ? "索引可用" : "尚未索引"}</small></span>
          </label>)}
        </div>
        <footer><span>{selectedIds.length} 篇已选择</span><button className="primary-button" onClick={() => setShowScope(false)}>完成</button></footer>
      </aside>}

      {showSettings && <aside className="rag-settings-panel">
        <header><h3>RAG 设置</h3><button aria-label="关闭 RAG 设置" onClick={() => setShowSettings(false)}>×</button></header>
        <section>
          <h4>Embedding 模型</h4>
          <label>服务商<select value={settings.provider} onChange={(event) => chooseProvider(event.target.value as RagProvider)}>
            <option value="openai">OpenAI</option>
            <option value="qwen">Qwen（阿里云百炼）</option>
            <option value="custom">自定义兼容 API</option>
          </select></label>
          <label>API Base URL<input value={settings.baseUrl} onChange={(event) => setSettings({ ...settings, baseUrl: event.target.value })} /></label>
          <label>Embedding 模型<input value={settings.model} onChange={(event) => setSettings({ ...settings, model: event.target.value })} /></label>
          {settings.provider === "qwen" && <p className="muted">默认使用百炼公共兼容端点；也可填入业务空间专属 Base URL。索引会按每批 10 个片段提交，以兼容不同百炼向量模型。</p>}
          <div className="rag-settings-grid">
            <label>每片字符数<input type="number" min={500} max={8000} value={settings.chunkChars}
              onChange={(event) => setSettings({ ...settings, chunkChars: Number(event.target.value) })} /></label>
            <label>召回片段数<input type="number" min={2} max={20} value={settings.topK}
              onChange={(event) => setSettings({ ...settings, topK: Number(event.target.value) })} /></label>
          </div>
          <label>API Key<input type="password" autoComplete="new-password" value={apiKey}
            placeholder={keyConfigured ? "已保存在 Reflo 数据库；留空保持不变" : "输入 Embedding API Key"}
            onChange={(event) => setApiKey(event.target.value)} /></label>
          <button className="primary-button" disabled={savingSettings || (!keyConfigured && !apiKey.trim())}
            onClick={() => void persistSettings()}>{savingSettings ? "保存中…" : "保存 Embedding 设置"}</button>
        </section>
        <section className="rag-answer-settings"><h4>回答模型</h4>
          <p className="muted">回答与翻译、Paper Tree 共用 Chat Completions 设置。</p><TranslationSettings />
        </section>
      </aside>}
    </div>
  );
}
