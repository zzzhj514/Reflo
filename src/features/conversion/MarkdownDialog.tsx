import { useEffect, useState } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import ReactMarkdown from "react-markdown";
import rehypeKatex from "rehype-katex";
import remarkGfm from "remark-gfm";
import remarkMath from "remark-math";
import "katex/dist/katex.min.css";
import type { Paper } from "../../shared/contracts/library";
import type { MarkdownDocument, MinerUPreferences } from "../../shared/contracts/conversion";
import { convertPdfToMarkdown, getMarkdownDocument, getMineruSettings, saveMineruSettings } from "../../shared/ipc/conversion";

type Props = { paper: Paper; onConverted?: () => void; onClose: () => void };

const DEFAULT_SETTINGS: MinerUPreferences = {
  model: "vlm", language: "en", enableOcr: false, enableFormula: true, enableTable: true,
};

const LANGUAGES: Array<[MinerUPreferences["language"], string]> = [
  ["en", "英文"], ["ch", "中文"], ["ja", "日文"], ["ko", "韩文"],
  ["fr", "法文"], ["de", "德文"], ["es", "西班牙文"],
];

export function MarkdownDialog({ paper, onConverted, onClose }: Props) {
  const [document, setDocument] = useState<MarkdownDocument | null>(null);
  const [loading, setLoading] = useState(true);
  const [converting, setConverting] = useState(false);
  const [saving, setSaving] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [mode, setMode] = useState<"preview" | "source">("preview");
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [apiToken, setApiToken] = useState("");
  const [tokenConfigured, setTokenConfigured] = useState(false);
  const [settings, setSettings] = useState<MinerUPreferences>(DEFAULT_SETTINGS);

  useEffect(() => {
    let active = true;
    Promise.all([getMarkdownDocument(paper.id), getMineruSettings()])
      .then(([markdown, mineru]) => {
        if (!active) return;
        setDocument(markdown);
        setSettings(mineru.preferences);
        setTokenConfigured(mineru.tokenConfigured);
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
      onConverted?.();
      setShowSettings(false);
      setMode("preview");
    } catch (cause) { setError(String(cause)); }
    finally { setConverting(false); }
  }

  function imageSource(source?: string) {
    if (!source) return source;
    const normalized = source.replace(/^\.\//, "");
    if (!normalized.startsWith("assets/") || normalized.split("/").includes("..")) return source;
    return convertFileSrc(`${document?.assetBasePath}/${normalized}`);
  }

  const busy = saving || converting;
  const canConvert = tokenConfigured || Boolean(apiToken.trim());

  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={(event) => {
      if (event.target === event.currentTarget && !busy) onClose();
    }}>
      <section className="markdown-dialog" role="dialog" aria-modal="true" aria-labelledby="markdown-dialog-title">
        <header>
          <div><h2 id="markdown-dialog-title">PDF 转 Markdown</h2><p title={paper.title}>{paper.title}</p></div>
          <button aria-label="关闭" disabled={busy} onClick={onClose}>×</button>
        </header>

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
              <span className="muted">{document.processor} · {document.model} · {document.updatedAt}</span>
              <div>
                <div className="markdown-mode" aria-label="Markdown 显示模式">
                  <button aria-pressed={mode === "preview"} onClick={() => setMode("preview")}>阅读</button>
                  <button aria-pressed={mode === "source"} onClick={() => setMode("source")}>源码</button>
                </div>
                <button className="secondary-button" onClick={() => setShowSettings(true)}>转换设置</button>
                <button className="secondary-button" onClick={() => {
                  void navigator.clipboard.writeText(document.markdown).then(() => {
                    setCopied(true); window.setTimeout(() => setCopied(false), 1500);
                  });
                }}>{copied ? "已复制" : "复制 Markdown"}</button>
              </div>
            </div>
            <p className="markdown-path">本地文件：{document.relativePath}</p>
            {mode === "source" ? <pre className="markdown-source">{document.markdown}</pre> : (
              <article className="markdown-preview">
                <ReactMarkdown remarkPlugins={[remarkGfm, remarkMath]} rehypePlugins={[rehypeKatex]}
                  components={{ img: ({ src, ...props }) => <img {...props} src={imageSource(src)} /> }}>
                  {document.markdown}
                </ReactMarkdown>
              </article>
            )}
          </div>
        )}
      </section>
    </div>
  );
}
