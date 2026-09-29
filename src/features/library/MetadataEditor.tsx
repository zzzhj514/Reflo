import { useRef, useState } from "react";
import type { FormEvent } from "react";
import type { Paper } from "../../shared/contracts/library";
import { updateMetadata } from "../../shared/ipc/library";

type Props = {
  paper: Paper;
  disabled?: boolean;
  onSaved: (paper: Paper) => void;
  onEditingChange: (editing: boolean) => void;
};

export function MetadataEditor({ paper, disabled = false, onSaved, onEditingChange }: Props) {
  const original = {
    title: paper.title,
    authors: paper.authors.join("\n"),
    year: paper.year?.toString() ?? "",
    doi: paper.doi ?? "",
    sourceUrl: paper.sourceUrl ?? "",
    venue: paper.venue ?? "",
    publisher: paper.publisher ?? "",
  };
  const [draft, setDraft] = useState(original);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const submitting = useRef(false);
  const dirty = JSON.stringify(draft) !== JSON.stringify(original);

  function change(field: keyof typeof draft, value: string) {
    const next = { ...draft, [field]: value };
    setDraft(next);
    setError(null);
    onEditingChange(JSON.stringify(next) !== JSON.stringify(original));
  }

  function reset() {
    setDraft(original);
    setError(null);
    onEditingChange(false);
  }

  async function save(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (disabled || submitting.current || !dirty) return;
    if (!draft.title.trim()) { setError("标题不能为空"); return; }
    const year = draft.year.trim() ? Number(draft.year) : null;
    if (year !== null && (!Number.isInteger(year) || year < 1 || year > 9999)) {
      setError("年份必须是 1 到 9999 之间的整数");
      return;
    }
    submitting.current = true;
    setSaving(true);
    setError(null);
    onEditingChange(true);
    try {
      const updated = await updateMetadata({
        id: paper.id,
        expectedRevision: paper.revision,
        title: draft.title,
        authors: draft.authors.split(/\r?\n/),
        year,
        doi: draft.doi || null,
        sourceUrl: draft.sourceUrl || null,
        venue: draft.venue || null,
        publisher: draft.publisher || null,
      });
      onEditingChange(false);
      onSaved(updated);
    } catch (cause) {
      setError(String(cause));
    } finally {
      submitting.current = false;
      setSaving(false);
    }
  }

  return (
    <form className="metadata-form" onSubmit={save}>
      <fieldset disabled={saving || disabled}>
        <label htmlFor="paper-title">标题</label>
        <textarea id="paper-title" required maxLength={2000} rows={3}
          value={draft.title} onChange={(event) => change("title", event.target.value)} />

        <label htmlFor="paper-authors">作者（每行一位，按署名顺序）</label>
        <textarea id="paper-authors" rows={4} value={draft.authors}
          onChange={(event) => change("authors", event.target.value)} />

        <label htmlFor="paper-year">年份</label>
        <input id="paper-year" type="number" min="1" max="9999" step="1"
          value={draft.year} onChange={(event) => change("year", event.target.value)} />

        <label htmlFor="paper-doi">DOI</label>
        <input id="paper-doi" value={draft.doi} placeholder="10.…"
          onChange={(event) => change("doi", event.target.value)} />

        <label htmlFor="paper-url">来源链接</label>
        <input id="paper-url" type="url" placeholder="https://…" value={draft.sourceUrl}
          onChange={(event) => change("sourceUrl", event.target.value)} />

        <label htmlFor="paper-venue">期刊 / 会议</label>
        <input id="paper-venue" value={draft.venue} placeholder="例如 Nature 或 NeurIPS"
          onChange={(event) => change("venue", event.target.value)} />

        <label htmlFor="paper-publisher">出版方</label>
        <input id="paper-publisher" value={draft.publisher} placeholder="例如 IEEE"
          onChange={(event) => change("publisher", event.target.value)} />

        <div className="metadata-actions">
          <button className="import-button" type="submit" disabled={!dirty}>
            {saving ? "保存中…" : "保存修改"}
          </button>
          <button className="secondary-button" type="button" disabled={!dirty} onClick={reset}>
            撤销修改
          </button>
        </div>
      </fieldset>
      {dirty && <p className="muted">有未保存的修改，请保存或撤销后再切换文献。</p>}
      {error && <p className="form-error" role="alert">{error}</p>}
    </form>
  );
}
