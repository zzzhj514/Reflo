import type { Annotation } from "../../../shared/contracts/reader";
import { TranslationSettings } from "./TranslationSettings";

export type InspectorState =
  | { mode: "list" }
  | { mode: "note"; selectedText: string; note: string }
  | { mode: "translation"; sourceText: string; translatedText: string | null; loading: boolean; error: string | null; truncated: boolean; provider: string | null; model: string | null }
  | { mode: "translation-settings" };

type Props = {
  state: InspectorState;
  annotations: Annotation[];
  busy: boolean;
  onClose: () => void;
  onNoteChange: (note: string) => void;
  onSaveNote: () => void;
  onDelete: (id: string) => void;
  onOpenPage: (pageIndex: number) => void;
  onOpenTranslationSettings: () => void;
};

export function ReaderInspector({ state, annotations, busy, onClose, onNoteChange, onSaveNote, onDelete, onOpenPage, onOpenTranslationSettings }: Props) {
  return (
    <aside className="reader-inspector" data-selection-ui aria-label="阅读批注面板">
      <header>
        <h3>{state.mode === "translation" ? "翻译" : state.mode === "translation-settings" ? "翻译设置" : state.mode === "note" ? "添加批注" : `批注与高亮 ${annotations.length}`}</h3>
        <button aria-label="关闭侧栏" onClick={onClose}>×</button>
      </header>
      {state.mode === "note" && (
        <div className="inspector-content">
          <blockquote>{state.selectedText}</blockquote>
          <label htmlFor="annotation-note">批注内容</label>
          <textarea id="annotation-note" autoFocus rows={7} value={state.note} onChange={(event) => onNoteChange(event.target.value)} />
          <button className="primary-button" disabled={busy || !state.note.trim()} onClick={onSaveNote}>
            {busy ? "保存中…" : "保存批注"}
          </button>
        </div>
      )}
      {state.mode === "translation" && (
        <div className="inspector-content">
          <p className="inspector-label">原文</p>
          <blockquote>{state.sourceText}</blockquote>
          <p className="inspector-label">译文</p>
          {state.loading && <p className="muted" role="status">正在翻译…</p>}
          {state.error && <p className="form-error" role="alert">{state.error}</p>}
          {state.error && <button className="secondary-button" onClick={onOpenTranslationSettings}>打开翻译设置</button>}
          {state.translatedText && <p className="translation-result">{state.translatedText}</p>}
          {state.truncated && <p className="muted">所选内容较长，本次仅翻译前 12,000 个字符。</p>}
          {state.provider && <p className="muted translation-provider">{state.provider} · {state.model}</p>}
        </div>
      )}
      {state.mode === "translation-settings" && (
        <div className="inspector-content"><TranslationSettings /></div>
      )}
      {state.mode === "list" && (
        <div className="annotation-list">
          {annotations.length === 0 && <p className="muted">还没有高亮或批注。</p>}
          {annotations.map((annotation) => (
            <article key={annotation.id} className="annotation-card">
              <button className="annotation-page" onClick={() => onOpenPage(annotation.pageIndex)}>
                第 {annotation.pageIndex + 1} 页 · {annotation.kind === "note" ? "批注" : "高亮"}
              </button>
              <blockquote>{annotation.selectedText}</blockquote>
              {annotation.note && <p>{annotation.note}</p>}
              <button className="danger-link" disabled={busy} onClick={() => onDelete(annotation.id)}>删除</button>
            </article>
          ))}
        </div>
      )}
    </aside>
  );
}
