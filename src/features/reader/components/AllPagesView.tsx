import { useEffect, useRef, useState } from "react";
import type { PDFDocumentProxy, RenderTask } from "../engine/pdf-engine";

const THUMBNAIL_WIDTH = 170;

type ThumbnailProps = {
  pdf: PDFDocumentProxy;
  pageIndex: number;
  active: boolean;
  onSelect: (pageIndex: number) => void;
};

function PageThumbnail({ pdf, pageIndex, active, onSelect }: ThumbnailProps) {
  const button = useRef<HTMLButtonElement>(null);
  const canvas = useRef<HTMLCanvasElement>(null);
  const [visible, setVisible] = useState(false);
  const [error, setError] = useState(false);

  useEffect(() => {
    const element = button.current;
    if (!element || visible) return;
    if (!("IntersectionObserver" in window)) {
      setVisible(true);
      return;
    }
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          setVisible(true);
          observer.disconnect();
        }
      },
      { rootMargin: "300px" },
    );
    observer.observe(element);
    return () => observer.disconnect();
  }, [visible]);

  useEffect(() => {
    if (!visible || !canvas.current) return;
    let cancelled = false;
    let task: RenderTask | undefined;
    async function render() {
      const page = await pdf.getPage(pageIndex + 1);
      if (cancelled || !canvas.current) return;
      const natural = page.getViewport({ scale: 1 });
      const viewport = page.getViewport({ scale: THUMBNAIL_WIDTH / natural.width });
      const ratio = Math.min(window.devicePixelRatio || 1, 2);
      const target = canvas.current;
      target.width = Math.max(1, Math.floor(viewport.width * ratio));
      target.height = Math.max(1, Math.floor(viewport.height * ratio));
      target.style.width = `${viewport.width}px`;
      target.style.height = `${viewport.height}px`;
      task = page.render({
        canvas: target,
        viewport,
        transform: [ratio, 0, 0, ratio, 0, 0],
      });
      await task.promise;
    }
    void render().catch(() => {
      if (!cancelled) setError(true);
    });
    return () => {
      cancelled = true;
      task?.cancel();
    };
  }, [pdf, pageIndex, visible]);

  return (
    <button
      ref={button}
      className="page-thumbnail"
      aria-current={active ? "page" : undefined}
      aria-label={`打开第 ${pageIndex + 1} 页`}
      onClick={() => onSelect(pageIndex)}
    >
      <span className="thumbnail-canvas-wrap">
        {!visible && <span className="thumbnail-placeholder" aria-hidden="true" />}
        {error && <span className="thumbnail-error">无法预览</span>}
        <canvas ref={canvas} aria-hidden="true" />
      </span>
      <span>第 {pageIndex + 1} 页</span>
    </button>
  );
}

type Props = {
  pdf: PDFDocumentProxy;
  activePage: number;
  onSelect: (pageIndex: number) => void;
};

export function AllPagesView({ pdf, activePage, onSelect }: Props) {
  const activeItem = useRef<HTMLDivElement>(null);

  useEffect(() => {
    activeItem.current?.scrollIntoView({ block: "center" });
  }, []);

  return (
    <div className="all-pages-scroll" aria-label={`全部 ${pdf.numPages} 页`}>
      <div className="all-pages-header">
        <strong>全部页面</strong>
        <span className="muted">共 {pdf.numPages} 页，点击缩略图打开</span>
      </div>
      <div className="page-thumbnail-grid">
        {Array.from({ length: pdf.numPages }, (_, pageIndex) => (
          <div key={pageIndex} ref={pageIndex === activePage ? activeItem : undefined}>
            <PageThumbnail
              pdf={pdf}
              pageIndex={pageIndex}
              active={pageIndex === activePage}
              onSelect={onSelect}
            />
          </div>
        ))}
      </div>
    </div>
  );
}
