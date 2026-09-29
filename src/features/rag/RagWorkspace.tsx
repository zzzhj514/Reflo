import { useEffect, useRef, useState } from "react";
import type { Paper } from "../../shared/contracts/library";
import type { RagIndexStatus, RagMessage, RagPreferences, RagProvider } from "../../shared/contracts/rag";
import { askPaperRag, getRagIndexStatus, getRagSettings, indexPaperRag, listRagMessages, saveRagSettings } from "../../shared/ipc/rag";
import { TranslationSettings } from "../reader/components/TranslationSettings";

type Props = { paper: Paper; onIndexed: () => void; onClose: () => void };

const DEFAULTS: RagPreferences = {
  provider: "openai", baseUrl: "https://api.openai.com/v1",
  model: "text-embedding-3-small", chunkChars: 2400, topK: 6,
};

export function RagWorkspace({ paper, onIndexed, onClose }: Props) {
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
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const endRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    let active = true;
    Promise.all([getRagSettings(), getRagIndexStatus(paper.id), listRagMessages(paper.id)])
      .then(([ragSettings, status, history]) => {
        if (!active) return;
        setSettings(ragSettings.preferences);
        setKeyConfigured(ragSettings.keyConfigured);
        setConfiguredProviders(ragSettings.configuredProviders);
        setIndexStatus(status);
        setMessages(history);
        if (!ragSettings.keyConfigured) setShowSettings(true);
      })
      .catch((cause) => { if (active) setError(`读取 RAG 数据失败：${String(cause)}`); })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [paper.id]);

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
    if (!value || asking || !indexStatus?.indexed) return;
    setAsking(true); setError(null); setQuestion("");
    setMessages((current) => [...current, {
      id: `pending-${Date.now()}`, role: "user", content: value, citations: [], createdAt: new Date().toISOString(),
    }]);
    try {
      await askPaperRag(paper.id, value);
      setMessages(await listRagMessages(paper.id));
    } catch (cause) {
      setMessages((current) => current.filter((item) => !item.id.startsWith("pending-")));
      setQuestion(value); setError(`问答失败：${String(cause)}`);
    } finally { setAsking(false); }
  }

  return (
    <div className="rag-workspace">
      <header className="rag-heading">
        <button className="secondary-button" disabled={indexing || asking} onClick={onClose}>返回文献库</button>
        <div><h2>论文问答</h2><p>{paper.title}</p></div>
        {indexStatus?.indexed && <span className="rag-index-badge">{indexStatus.chunkCount} 个片段</span>}
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
              <h3>{indexStatus?.indexed ? "向这篇论文提问" : "先为论文创建检索索引"}</h3>
              <p>{indexStatus?.indexed
                ? "回答只使用当前论文的 Markdown 片段，并附上检索来源。"
                : paper.hasMarkdown ? "配置 Embedding API 后点击右上角“创建索引”。" : "请先将 PDF 转换为 Markdown。"}</p>
              {indexStatus?.indexed && <div className="rag-suggestions">
                {["这篇论文解决了什么问题？", "核心方法和技术贡献是什么？", "实验如何证明方法有效？", "作者提到了哪些局限？"].map((item) =>
                  <button key={item} onClick={() => setQuestion(item)}>{item}</button>)}
              </div>}
            </section>
          )}
          <div className="rag-message-list">
            {messages.map((message) => <article key={message.id} className={`rag-message ${message.role}`}>
              <header>{message.role === "user" ? "你" : "Reflo"}</header>
              <p>{message.content}</p>
              {message.citations.length > 0 && <div className="rag-citations">
                {message.citations.map((citation) => <details key={citation.chunkId}>
                  <summary>[{citation.number}] {citation.headingPath}</summary>
                  <blockquote>{citation.excerpt}</blockquote>
                </details>)}
              </div>}
            </article>)}
            {asking && <article className="rag-message assistant pending"><header>Reflo</header><p>正在检索论文并组织回答…</p></article>}
            <div ref={endRef} />
          </div>
        </main>
      )}

      <footer className="rag-composer">
        <textarea value={question} disabled={asking || !indexStatus?.indexed} rows={2}
          placeholder={indexStatus?.indexed ? "针对这篇论文提问…（⌘/Ctrl + Enter 发送）" : "创建索引后即可提问"}
          onChange={(event) => setQuestion(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) { event.preventDefault(); void ask(); }
          }} />
        <button className="primary-button" disabled={asking || !question.trim() || !indexStatus?.indexed}
          onClick={() => void ask()}>{asking ? "回答中…" : "发送"}</button>
      </footer>

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
