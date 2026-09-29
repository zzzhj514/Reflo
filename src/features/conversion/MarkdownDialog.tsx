import { useEffect, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import ReactMarkdown from "react-markdown";
import rehypeKatex from "rehype-katex";
import remarkGfm from "remark-gfm";
import remarkMath from "remark-math";
import "katex/dist/katex.min.css";
import type { Paper } from "../../shared/contracts/library";
import type { MarkdownDocument, MinerUPreferences } from "../../shared/contracts/conversion";
import type { MarkdownTranslationDocument } from "../../shared/contracts/translation";
import { convertPdfToMarkdown, getMarkdownDocument, getMineruSettings, saveMineruSettings } from "../../shared/ipc/conversion";
import { getMarkdownTranslation, translateMarkdownDocument } from "../../shared/ipc/translation";
import { TranslationSettings } from "../reader/components/TranslationSettings";

type Props = { paper: Paper; onConverted?: () => void; onTranslated?: () => void; onClose: () => void };

const DEFAULT_SETTINGS: MinerUPreferences = {
  model: "vlm", language: "en", enableOcr: false, enableFormula: true, enableTable: true,
};

const LANGUAGES: Array<[MinerUPreferences["language"], string]> = [
  ["en", "英文"], ["ch", "中文"], ["ja", "日文"], ["ko", "韩文"],
  ["fr", "法文"], ["de", "德文"], ["es", "西班牙文"],
];

export function MarkdownWorkspace({ paper, onConverted, onTranslated, onClose }: Props) {
  const [document, setDocument] = useState<MarkdownDocument | null>(null);
  const [translation, setTranslation] = useState<MarkdownTranslationDocument | null>(null);
  const [loading, setLoading] = useState(true);
  const [converting, setConverting] = useState(false);
  const [translating, setTranslating] = useState(false);
  const [saving, setSaving] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [showTranslationSettings, setShowTranslationSettings] = useState(false);
  const [mode, setMode] = useState<"preview" | "source">("preview");
  const [version, setVersion] = useState<"original" | "translated">("original");
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [apiToken, setApiToken] = useState("");
  const [tokenConfigured, setTokenConfigured] = useState(false);
  const [settings, setSettings] = useState<MinerUPreferences>(DEFAULT_SETTINGS);

  useEffect(() => {
    let active = true;
    Promise.all([getMarkdownDocument(paper.id), getMineruSettings(), getMarkdownTranslation(paper.id)])
      .then(([markdown, mineru, translated]) => {
        if (!active) return;
        setDocument(markdown);
        setSettings(mineru.preferences);
        setTokenConfigured(mineru.tokenConfigured);
        setTranslation(translated);
        setShowSettings(!markdown);
      })
      .catch((cause) => {
        if (active) {
          setError(`读取转换信息失败：${String(cause)}`);
          setShowSettings(true);
        }
      })
      .finally(() => { if (active) setLoading(false); });
    return () => { active = false; };
  }, [paper.id]);

  async function persistSettings() {
    const result = await saveMineruSettings(settings, apiToken || undefined);
    setTokenConfigured(result.tokenConfigured);
    setSettings(result.preferences);
    setApiToken("");
  }

  async function saveOnly() {
    if (saving || converting) return;
    setSaving(true);
    setError(null);
    try { await persistSettings(); } catch (cause) { setError(String(cause)); }
    finally { setSaving(false); }
  }

  async function convert() {
    if ((!tokenConfigured && !apiToken.trim()) || converting) return;
    setConverting(true);
    setError(null);
    try {
      await persistSettings();
      const result = await convertPdfToMarkdown(paper.id);
      setDocument(result);
      setTranslation(null);
      setVersion("original");
      onConverted?.();
      setShowSettings(false);
      setMode("preview");
    } catch (cause) { setError(String(cause)); }
    finally { setConverting(false); }
  }

  async function translateWholeDocument() {
    if (!document || translating) return;
    setTranslating(true);
    setError(null);
    try {
      const result = await translateMarkdownDocument(paper.id);
      setTranslation(result);
      onTranslated?.();
      setVersion("translated");
      setShowTranslationSettings(false);
    } catch (cause) {
      setError(String(cause));
    } finally {
      setTranslating(false);
    }
  }

  function imageSource(source?: string) {
    if (!source) return source;
    const normalized = source.replace(/^\.\//, "");
    if (!normalized.startsWith("assets/") || normalized.split("/").includes("..")) return source;
    return convertFileSrc(`${activeDocument?.assetBasePath}/${normalized}`);
  }

  const activeDocument = version === "translated" && translation ? translation : document;
  const busy = saving || converting || translating;
  const canConvert = tokenConfigured || Boolean(apiToken.trim());

  return (
    <div className="markdown-workspace" aria-labelledby="markdown-workspace-title">
      <header className="markdown-heading">
        <button className="secondary-button" disabled={busy} onClick={onClose}>返回文献库</button>
        <div>
          <h2 id="markdown-workspace-title">Markdown</h2>
          <p title={paper.title}>{paper.title}</p>
        </div>
      </header>
      <main className="markdown-workspace-body">

        {loading && <p className="dialog-status" role="status">正在读取转换记录…</p>}
        {error && <p className="form-error dialog-error" role="alert">{error}</p>}

        {!loading && showSettings && (
          <div className="mineru-settings">
            <label htmlFor="mineru-token">MinerU API Token</label>
            <input id="mineru-token" type="password" autoComplete="new-password" value={apiToken}
              placeholder={tokenConfigured ? "已保存在 macOS 钥匙串；留空保持不变" : "从 mineru.net 获取的 Token"}
              onChange={(event) => setApiToken(event.target.value)} />
            <p className="muted">{tokenConfigured ? "Token 已安全保存。输入新值可替换。" : "保存后由 macOS 钥匙串加密管理，Reflo 不会把明文写入数据库。"}</p>

            <div className="mineru-grid">
              <label>模型<select value={settings.model} onChange={(event) => setSettings({ ...settings, model: event.target.value as MinerUPreferences["model"] })}>
                <option value="vlm">VLM（推荐）</option><option value="pipeline">Pipeline</option>
              </select></label>
              <label>文档语言<select value={settings.language} onChange={(event) => setSettings({ ...settings, language: event.target.value as MinerUPreferences["language"] })}>
                {LANGUAGES.map(([value, label]) => <option key={value} value={value}>{label}</option>)}
              </select></label>
            </div>
            <label className="check-row"><input type="checkbox" checked={settings.enableOcr} onChange={(event) => setSettings({ ...settings, enableOcr: event.target.checked })} />扫描件 OCR</label>
            <label className="check-row"><input type="checkbox" checked={settings.enableFormula} onChange={(event) => setSettings({ ...settings, enableFormula: event.target.checked })} />识别公式</label>
            <label className="check-row"><input type="checkbox" checked={settings.enableTable} onChange={(event) => setSettings({ ...settings, enableTable: event.target.checked })} />识别表格</label>
            <div className="dialog-actions">
              {document && <button className="secondary-button" disabled={busy} onClick={() => setShowSettings(false)}>取消</button>}
              <button className="secondary-button" disabled={busy || (!tokenConfigured && !apiToken.trim())} onClick={() => void saveOnly()}>{saving ? "保存中…" : "保存设置"}</button>
              <button className="import-button" disabled={busy || !canConvert} onClick={() => void convert()}>{converting ? "MinerU 解析中…" : document ? "重新转换" : "开始转换"}</button>
            </div>
            {converting && <p className="muted" role="status">正在上传并解析 PDF，长文档可能需要几分钟，请保持窗口开启。</p>}
          </div>
        )}

        {!loading && document && !showSettings && (
          <div className="markdown-result">
            <div className="markdown-result-toolbar">
              <span className="muted">{version === "translated" && translation
                ? `译文 · ${translation.provider} · ${translation.model} · ${translation.updatedAt}`
                : `${document.processor} · ${document.model} · ${document.updatedAt}`}</span>
              <div>
                {translation && <div className="markdown-mode" aria-label="Markdown 文档版本">
                  <button aria-pressed={version === "original"} onClick={() => setVersion("original")}>原文</button>
                  <button aria-pressed={version === "translated"} onClick={() => setVersion("translated")}>译文</button>
                </div>}
                <div className="markdown-mode" aria-label="Markdown 显示模式">
                  <button aria-pressed={mode === "preview"} onClick={() => setMode("preview")}>阅读</button>
                  <button aria-pressed={mode === "source"} onClick={() => setMode("source")}>源码</button>
                </div>
                <button className="primary-button" disabled={busy} onClick={() => void translateWholeDocument()}>
                  {translating ? "正在翻译全文…" : translation ? "重新翻译" : "一键翻译"}
                </button>
                <button className="secondary-button" disabled={busy} onClick={() => setShowTranslationSettings(true)}>翻译设置</button>
                <button className="secondary-button" onClick={() => setShowSettings(true)}>转换设置</button>
                <button className="secondary-button" onClick={() => {
                  void navigator.clipboard.writeText(activeDocument?.markdown ?? "").then(() => {
                    setCopied(true); window.setTimeout(() => setCopied(false), 1500);
                  });
                }}>{copied ? "已复制" : "复制 Markdown"}</button>
              </div>
            </div>
            {translating && <p className="markdown-translation-progress" role="status">正在按段落翻译整篇 Markdown，长文档可能需要几分钟，请保持窗口开启。</p>}
            <p className="markdown-path">本地文件：{activeDocument?.relativePath}</p>
            {mode === "source" ? <pre className="markdown-source">{activeDocument?.markdown}</pre> : (
              <article className="markdown-preview">
                <ReactMarkdown remarkPlugins={[remarkGfm, remarkMath]} rehypePlugins={[rehypeKatex]}
                  components={{ img: ({ src, ...props }) => <img {...props} src={imageSource(src)} /> }}>
                  {activeDocument?.markdown}
                </ReactMarkdown>
              </article>
            )}
          </div>
        )}
        {showTranslationSettings && (
          <aside className="markdown-translation-settings" aria-label="翻译设置">
            <header><h3>翻译设置</h3><button aria-label="关闭翻译设置" onClick={() => setShowTranslationSettings(false)}>×</button></header>
            <TranslationSettings />
          </aside>
        )}
      </main>
    </div>
  );
}
