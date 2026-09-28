import { useEffect, useRef, useState } from "react";
import type { WheelEvent } from "react";
import type { Annotation } from "../../../shared/contracts/reader";
import { TextLayer } from "../engine/pdf-engine";
import type { PDFDocumentProxy, RenderTask } from "../engine/pdf-engine";

type PageProps = { pdf: PDFDocumentProxy; pageIndex: number; scale: number; annotations: Annotation[] };

export function PdfPage({ pdf, pageIndex, scale, annotations }: PageProps) {
  const host = useRef<HTMLDivElement>(null);
  const [error, setError] = useState<string | null>(null);
  const [rendering, setRendering] = useState(true);

  useEffect(() => {
    const container = host.current!;
    let cancelled = false;
    let renderTask: RenderTask | undefined;
    let textLayer: TextLayer | undefined;
    setRendering(true);
    setError(null);
    container.replaceChildren();

    async function render() {
      const page = await pdf.getPage(pageIndex + 1);
      if (cancelled) return;
      const viewport = page.getViewport({ scale });
      const sheet = document.createElement("div");
      sheet.className = "pdf-sheet";
      sheet.style.width = `${viewport.width}px`;
      sheet.style.height = `${viewport.height}px`;
      sheet.style.setProperty("--scale-factor", String(scale));
      sheet.style.setProperty("--total-scale-factor", String(scale));
      const canvas = document.createElement("canvas");
      canvas.setAttribute("aria-label", `PDF 第 ${pageIndex + 1} 页`);
      const ratio = Math.min(
        window.devicePixelRatio || 1,
        2,
        8192 / Math.max(viewport.width, viewport.height),
        Math.sqrt(16_000_000 / (viewport.width * viewport.height)),
      );
      canvas.width = Math.max(1, Math.floor(viewport.width * ratio));
      canvas.height = Math.max(1, Math.floor(viewport.height * ratio));
      canvas.style.width = "100%";
      canvas.style.height = "100%";
      const text = document.createElement("div");
      text.className = "textLayer";
      const highlights = document.createElement("div");
      highlights.className = "annotation-layer";
      for (const annotation of annotations) {
        if (annotation.pageIndex !== pageIndex) continue;
        for (const rect of annotation.rects) {
          const mark = document.createElement("span");
          mark.className = `annotation-mark annotation-${annotation.kind}`;
          mark.style.left = `${rect.x * 100}%`;
          mark.style.top = `${rect.y * 100}%`;
          mark.style.width = `${rect.width * 100}%`;
          mark.style.height = `${rect.height * 100}%`;
          mark.style.backgroundColor = `${annotation.color}99`;
          mark.title = annotation.note || annotation.selectedText;
          highlights.append(mark);
        }
      }
      sheet.append(canvas, highlights, text);
      container.append(sheet);
      renderTask = page.render({
        canvas,
        viewport,
        transform: [ratio, 0, 0, ratio, 0, 0],
      });
      await renderTask.promise;
      if (cancelled) return;
      textLayer = new TextLayer({
        textContentSource: page.streamTextContent(),
        container: text,
        viewport,
      });
      await textLayer.render();
      if (!cancelled) setRendering(false);
    }

    void render().catch((cause) => {
      if (!cancelled) {
        setError(`第 ${pageIndex + 1} 页渲染失败：${String(cause)}`);
        setRendering(false);
      }
    });
    return () => {
      cancelled = true;
      renderTask?.cancel();
      textLayer?.cancel();
      container.replaceChildren();
    };
  }, [pdf, pageIndex, scale, annotations]);

  return (
    <div className="pdf-page-frame" data-page-index={pageIndex}>
      {rendering && <p role="status" className="reader-page-notice">正在渲染第 {pageIndex + 1} 页…</p>}
      {error && <p role="alert" className="form-error reader-page-notice">{error}</p>}
      <div ref={host} />
    </div>
  );
}

type Props = {
  pdf: PDFDocumentProxy;
  pageIndices: number[];
  scale: number;
  annotations: Annotation[];
  onWheel: (event: WheelEvent<HTMLDivElement>) => void;
};

export function PdfViewport({ pdf, pageIndices, scale, annotations, onWheel }: Props) {
  return (
    <div className="pdf-scroll" onWheel={onWheel} tabIndex={0} aria-label="PDF 页面，滚轮翻页">
      <div className="pdf-page-host">
        {pageIndices.map((pageIndex) => (
          <PdfPage
            key={`${pageIndex}:${scale}`}
            pdf={pdf}
            pageIndex={pageIndex}
            scale={scale}
            annotations={annotations}
          />
        ))}
      </div>
    </div>
  );
}
