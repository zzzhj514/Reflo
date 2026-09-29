import { useEffect, useRef, useState } from "react";
import type { KeyboardEvent, MouseEvent, WheelEvent } from "react";
import type { Annotation } from "../../../shared/contracts/reader";
import { translateText } from "../../../shared/ipc/translation";
import {
  createAnnotation,
  deleteAnnotation,
  listAnnotations,
  openDocument,
  saveReadingPosition,
} from "../../../shared/ipc/reader";
import { loadPdf } from "../engine/pdf-engine";
import type { PDFDocumentProxy } from "../engine/pdf-engine";
import { AllPagesView } from "./AllPagesView";
import { ContinuousPdfView } from "./ContinuousPdfView";
import { PdfViewport } from "./PdfViewport";
import { ReaderInspector } from "./ReaderInspector";
import type { InspectorState } from "./ReaderInspector";
import { SelectionToolbar } from "./SelectionToolbar";
import type { TextSelection } from "./SelectionToolbar";
import "../text-layer.css";

type Props = { paperId: string; title: string; onClose: () => void };
type ViewMode = "single" | "double" | "continuous" | "all";

const ZOOM_LEVELS = [0.25, 0.5, 0.75, 1, 1.25, 1.5, 2, 3];

export function ReaderWorkspace({ paperId, title, onClose }: Props) {
  const [pdf, setPdf] = useState<PDFDocumentProxy | null>(null);
  const [pageIndex, setPageIndex] = useState(0);
  const [pageInput, setPageInput] = useState("1");
  const [scale, setScale] = useState(1);
  const [viewMode, setViewMode] = useState<ViewMode>("single");
  const [error, setError] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [retry, setRetry] = useState(0);
  const [savePending, setSavePending] = useState(false);
  const [annotations, setAnnotations] = useState<Annotation[]>([]);
  const [selection, setSelection] = useState<TextSelection | null>(null);
  const [inspector, setInspector] = useState<InspectorState | null>(null);
  const [annotationBusy, setAnnotationBusy] = useState(false);
  const [annotationError, setAnnotationError] = useState<string | null>(null);
  const documentId = useRef<string | null>(null);
  const saveSequence = useRef(0);
  const wheelTotal = useRef(0);
  const wheelLocked = useRef(false);
  const wheelTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const continuousSaveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [continuousJump, setContinuousJump] = useState<{ pageIndex: number; sequence: number } | null>(null);

  useEffect(() => () => {
    if (wheelTimer.current) clearTimeout(wheelTimer.current);
    if (continuousSaveTimer.current) clearTimeout(continuousSaveTimer.current);
  }, []);

  useEffect(() => {
    let cancelled = false;
    let task: ReturnType<typeof loadPdf> | undefined;
    documentId.current = null;
    setPdf(null);
    setError(null);
    setAnnotations([]);
    setSelection(null);
    setInspector(null);
    async function open() {
      const record = await openDocument(paperId);
      if (cancelled) return;
      task = loadPdf(record.url);
      // Password-protected PDFs get an actionable error rather than an endless spinner.
      task.onPassword = () => {
        if (!cancelled) setError("此 PDF 需要密码，请先使用其他阅读器解密后再导入。");
        void task?.destroy();
      };
      const [loaded, savedAnnotations] = await Promise.all([
        task.promise,
        listAnnotations(record.documentId),
      ]);
      if (cancelled) return;
      documentId.current = record.documentId;
      const page = Math.min(record.pageIndex, loaded.numPages - 1);
      setPageIndex(page);
      setPageInput(String(page + 1));
      setScale(record.scale);
      setAnnotations(savedAnnotations);
      setPdf(loaded);
    }
    void open().catch((cause) => {
      if (!cancelled) setError((current) => current ?? `无法打开 PDF：${String(cause)}`);
    });
    return () => { cancelled = true; void task?.destroy(); };
  }, [paperId, retry]);

  function persist(page: number, zoom: number) {
    if (!documentId.current) return;
    const sequence = ++saveSequence.current;
    setSavePending(true);
    setSaveError(null);
    void saveReadingPosition(documentId.current, page, zoom)
      .catch((cause) => { if (sequence === saveSequence.current) setSaveError(`阅读位置保存失败：${String(cause)}`); })
      .finally(() => { if (sequence === saveSequence.current) setSavePending(false); });
  }

  const pageStep = viewMode === "double" ? 2 : 1;

  function changePage(next: number) {
    if (!pdf) return;
    const page = Math.max(0, Math.min(pdf.numPages - 1, next));
    setPageIndex(page);
    setPageInput(String(page + 1));
    persist(page, scale);
    if (viewMode === "continuous") {
      setContinuousJump((current) => ({ pageIndex: page, sequence: (current?.sequence ?? 0) + 1 }));
    }
  }

  function trackContinuousPage(page: number) {
    setPageIndex(page);
    setPageInput(String(page + 1));
    if (continuousSaveTimer.current) clearTimeout(continuousSaveTimer.current);
    continuousSaveTimer.current = setTimeout(() => persist(page, scale), 300);
  }

  function changeScale(next: number) {
    const zoom = Math.max(0.25, Math.min(3, next));
    setScale(zoom);
    persist(pageIndex, zoom);
  }

  function stepScale(direction: -1 | 1) {
    const current = ZOOM_LEVELS.findIndex((zoom) => zoom >= scale - 0.001);
    const index = Math.max(0, Math.min(ZOOM_LEVELS.length - 1, current + direction));
    changeScale(ZOOM_LEVELS[index]);
  }

  function handleWheel(event: WheelEvent<HTMLDivElement>) {
    if (selection) {
      window.getSelection()?.removeAllRanges();
      setSelection(null);
    }
    if (!pdf || viewMode === "all" || Math.abs(event.deltaY) < Math.abs(event.deltaX)) return;
    if (viewMode === "continuous" && !event.metaKey && !event.ctrlKey) return;
    event.preventDefault();
    if (wheelTimer.current) clearTimeout(wheelTimer.current);
    wheelTimer.current = setTimeout(() => {
      wheelLocked.current = false;
      wheelTotal.current = 0;
    }, 220);
    if (wheelLocked.current) return;
    wheelTotal.current += event.deltaY;
    if (Math.abs(wheelTotal.current) < 60) return;
    wheelLocked.current = true;
    if (event.metaKey || event.ctrlKey) {
      stepScale(wheelTotal.current > 0 ? -1 : 1);
    } else {
      changePage(pageIndex + (wheelTotal.current > 0 ? pageStep : -pageStep));
    }
  }

  const pageIndices = pdf
    ? [pageIndex, ...(viewMode === "double" && pageIndex + 1 < pdf.numPages ? [pageIndex + 1] : [])]
    : [];

  function openPage(page: number) {
    setViewMode("single");
    changePage(page);
  }

  function captureSelection(event: MouseEvent<HTMLElement> | KeyboardEvent<HTMLElement>) {
    const target = event.target;
    if (target instanceof Element && target.closest("[data-selection-ui]")) return;
    window.setTimeout(() => {
      const nativeSelection = window.getSelection();
      if (!nativeSelection || nativeSelection.isCollapsed || nativeSelection.rangeCount === 0) {
        setSelection(null);
        return;
      }
      const range = nativeSelection.getRangeAt(0);
      const nodeElement = (node: Node | null) => node instanceof Element ? node : node?.parentElement ?? null;
      const startPage = nodeElement(range.startContainer)?.closest<HTMLElement>(".pdf-page-frame");
      const endPage = nodeElement(range.endContainer)?.closest<HTMLElement>(".pdf-page-frame");
      const sheet = startPage?.querySelector<HTMLElement>(".pdf-sheet");
      if (!startPage || startPage !== endPage || !sheet) {
        setSelection(null);
        return;
      }
      const sheetRect = sheet.getBoundingClientRect();
      const clientRects = Array.from(range.getClientRects()).filter((rect) =>
        rect.width > 0 && rect.height > 0 && rect.bottom > sheetRect.top && rect.top < sheetRect.bottom,
      );
      const selectedText = nativeSelection.toString().trim();
      if (clientRects.length === 0 || !selectedText) {
        setSelection(null);
        return;
      }
      const left = Math.min(...clientRects.map((rect) => rect.left));
      const right = Math.max(...clientRects.map((rect) => rect.right));
      const top = Math.min(...clientRects.map((rect) => rect.top));
      const bottom = Math.max(...clientRects.map((rect) => rect.bottom));
      const toolbarY = top > 58 ? top - 46 : bottom + 10;
      setSelection({
        pageIndex: Number(startPage.dataset.pageIndex),
        text: selectedText,
        rects: clientRects.map((rect) => ({
          x: Math.max(0, rect.left - sheetRect.left) / sheetRect.width,
          y: Math.max(0, rect.top - sheetRect.top) / sheetRect.height,
          width: Math.min(rect.right, sheetRect.right) - Math.max(rect.left, sheetRect.left),
          height: Math.min(rect.bottom, sheetRect.bottom) - Math.max(rect.top, sheetRect.top),
        })).map((rect) => ({
          ...rect,
          width: rect.width / sheetRect.width,
          height: rect.height / sheetRect.height,
        })),
        viewportRects: clientRects.map((rect) => ({
          left: rect.left,
          top: rect.top,
          width: rect.width,
          height: rect.height,
        })),
        position: {
          x: Math.max(150, Math.min(window.innerWidth - 150, (left + right) / 2)),
          y: Math.max(8, Math.min(window.innerHeight - 48, toolbarY)),
        },
      });
    });
  }

  function clearSelection() {
    window.getSelection()?.removeAllRanges();
    setSelection(null);
  }

  async function saveSelectedAnnotation(kind: "highlight" | "note", note: string | null = null) {
    if (!selection || !documentId.current) return;
    setAnnotationBusy(true);
    setAnnotationError(null);
    try {
      const saved = await createAnnotation({
        documentId: documentId.current,
        pageIndex: selection.pageIndex,
        kind,
        selectedText: selection.text,
        note,
        rects: selection.rects,
        color: kind === "note" ? "#fb923c" : "#fde047",
      });
      setAnnotations((current) => [...current, saved].sort((a, b) => a.pageIndex - b.pageIndex));
      setInspector(kind === "note" ? { mode: "list" } : inspector);
      clearSelection();
    } catch (cause) {
      setAnnotationError(`保存失败：${String(cause)}`);
    } finally {
      setAnnotationBusy(false);
    }
  }

  function startTranslation() {
    if (!selection) return;
    const sourceText = selection.text;
    setInspector({ mode: "translation", sourceText, translatedText: null, loading: true, error: null, truncated: false, provider: null, model: null });
    void translateText(sourceText).then((result) => {
      setInspector({
        mode: "translation",
        sourceText: result.sourceText,
        translatedText: result.translatedText,
        loading: false,
        error: null,
        truncated: result.truncated,
        provider: result.provider,
        model: result.model,
      });
    }).catch((cause) => {
      setInspector({ mode: "translation", sourceText, translatedText: null, loading: false, error: String(cause), truncated: false, provider: null, model: null });
    });
  }

  async function removeAnnotation(id: string) {
    setAnnotationBusy(true);
    setAnnotationError(null);
    try {
      await deleteAnnotation(id);
      setAnnotations((current) => current.filter((annotation) => annotation.id !== id));
    } catch (cause) {
      setAnnotationError(`删除失败：${String(cause)}`);
    } finally {
      setAnnotationBusy(false);
    }
  }

  function openAnnotationPage(page: number) {
    setViewMode("single");
    changePage(page);
  }

  return (
    <section className="reader-workspace" aria-label="PDF 阅读器" onMouseUp={captureSelection} onKeyUp={captureSelection}>
      <header className="reader-heading">
        <button className="secondary-button" onClick={onClose}>返回文献库</button>
        <h2 title={title}>{title}</h2>
      </header>
      {pdf && !error && (
        <>
          <div className="reader-toolbar">
            {viewMode !== "all" && (
              <>
                <button disabled={pageIndex === 0} onClick={() => changePage(pageIndex - pageStep)}>上一页</button>
                <form onSubmit={(event) => {
                  event.preventDefault();
                  const page = Number(pageInput);
                  if (Number.isInteger(page) && page >= 1 && page <= pdf.numPages) changePage(page - 1);
                  else setPageInput(String(pageIndex + 1));
                }}>
                  <label htmlFor="pdf-page">页码</label>
                  <input id="pdf-page" type="number" min={1} max={pdf.numPages} value={pageInput}
                    onChange={(event) => setPageInput(event.target.value)} />
                  <span>/ {pdf.numPages}</span>
                  <button type="submit">跳转</button>
                </form>
                <button disabled={pageIndex >= pdf.numPages - pageStep} onClick={() => changePage(pageIndex + pageStep)}>下一页</button>
                <label htmlFor="pdf-zoom">缩放</label>
                <button aria-label="缩小" disabled={scale <= ZOOM_LEVELS[0]} onClick={() => stepScale(-1)}>−</button>
                <select id="pdf-zoom" value={scale} onChange={(event) => changeScale(Number(event.target.value))}>
                  {ZOOM_LEVELS.map((zoom) => <option key={zoom} value={zoom}>{zoom * 100}%</option>)}
                </select>
                <button aria-label="放大" disabled={scale >= ZOOM_LEVELS.at(-1)!} onClick={() => stepScale(1)}>＋</button>
              </>
            )}
            <div className="view-mode" role="group" aria-label="页面布局">
              <button aria-pressed={viewMode === "single"} onClick={() => setViewMode("single")}>单页</button>
              <button aria-pressed={viewMode === "double"} onClick={() => setViewMode("double")}>双页</button>
              <button aria-pressed={viewMode === "continuous"} onClick={() => setViewMode("continuous")}>连续滚动</button>
              <button aria-pressed={viewMode === "all"} onClick={() => setViewMode("all")}>全部页面</button>
            </div>
            <button className="annotation-toggle" data-selection-ui onClick={() => setInspector({ mode: "list" })}>
              批注 {annotations.length}
            </button>
            <button data-selection-ui onClick={() => setInspector({ mode: "translation-settings" })}>翻译设置</button>
            <span className="muted" role="status">{savePending ? "保存位置中…" : "阅读位置自动保存"}</span>
            <span className="muted reader-wheel-help">
              {viewMode === "all"
                ? "滚动浏览全部页面"
                : viewMode === "continuous"
                  ? "滚轮连续滚动 · ⌘/Ctrl + 滚轮缩放"
                  : "滚轮翻页 · ⌘/Ctrl + 滚轮缩放"}
            </span>
          </div>
          {(saveError || annotationError) && <p className="form-error reader-error" role="alert">{saveError || annotationError}</p>}
          {viewMode === "all" ? (
            <AllPagesView pdf={pdf} activePage={pageIndex} onSelect={openPage} />
          ) : viewMode === "continuous" ? (
            <ContinuousPdfView
              pdf={pdf}
              activePage={pageIndex}
              scale={scale}
              annotations={annotations}
              jumpTarget={continuousJump}
              onActivePageChange={trackContinuousPage}
              onWheel={handleWheel}
            />
          ) : (
            <PdfViewport pdf={pdf} pageIndices={pageIndices} scale={scale} annotations={annotations} onWheel={handleWheel} />
          )}
        </>
      )}
      {selection && !inspector && (
        <SelectionToolbar
          selection={selection}
          busy={annotationBusy}
          onHighlight={() => void saveSelectedAnnotation("highlight")}
          onNote={() => setInspector({ mode: "note", selectedText: selection.text, note: "" })}
          onTranslate={startTranslation}
          onCopy={() => {
            void navigator.clipboard.writeText(selection.text).catch((cause) => setAnnotationError(`复制失败：${String(cause)}`));
            clearSelection();
          }}
        />
      )}
      {selection && (
        <div className="selection-location-layer" aria-hidden="true">
          {selection.viewportRects.map((rect, index) => (
            <span
              key={index}
              style={{ left: rect.left, top: rect.top, width: rect.width, height: rect.height }}
            />
          ))}
        </div>
      )}
      {inspector && (
        <ReaderInspector
          state={inspector}
          annotations={annotations}
          busy={annotationBusy}
          onClose={() => { setInspector(null); clearSelection(); }}
          onNoteChange={(note) => setInspector((current) => current?.mode === "note" ? { ...current, note } : current)}
          onSaveNote={() => {
            if (inspector.mode === "note") void saveSelectedAnnotation("note", inspector.note);
          }}
          onDelete={(id) => void removeAnnotation(id)}
          onOpenPage={openAnnotationPage}
          onOpenTranslationSettings={() => setInspector({ mode: "translation-settings" })}
        />
      )}
      {!pdf && !error && <p className="reader-notice" role="status">正在加载 PDF…</p>}
      {error && <div className="reader-notice" role="alert"><p>{error}</p><button onClick={() => setRetry((value) => value + 1)}>重新加载</button></div>}
    </section>
  );
}
