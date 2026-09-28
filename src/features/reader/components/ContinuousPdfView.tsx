import { useEffect, useRef, useState } from "react";
import type { WheelEvent } from "react";
import type { Annotation } from "../../../shared/contracts/reader";
import type { PDFDocumentProxy } from "../engine/pdf-engine";
import { PdfPage } from "./PdfViewport";

type LazyPageProps = {
  pdf: PDFDocumentProxy;
  pageIndex: number;
  scale: number;
  annotations: Annotation[];
};

function LazyPage({ pdf, pageIndex, scale, annotations }: LazyPageProps) {
  const frame = useRef<HTMLDivElement>(null);
  const [nearViewport, setNearViewport] = useState(false);

  useEffect(() => {
    const element = frame.current;
    if (!element || nearViewport) return;
    if (!("IntersectionObserver" in window)) {
      setNearViewport(true);
      return;
    }
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          setNearViewport(true);
          observer.disconnect();
        }
      },
      { rootMargin: "1200px 0px" },
    );
    observer.observe(element);
    return () => observer.disconnect();
  }, [nearViewport]);

  return (
    <div
      ref={frame}
      className="continuous-page"
      data-page-index={pageIndex}
      aria-label={`第 ${pageIndex + 1} 页`}
      style={{ minWidth: `${612 * scale}px`, minHeight: `${792 * scale}px` }}
    >
      {nearViewport ? (
        <PdfPage pdf={pdf} pageIndex={pageIndex} scale={scale} annotations={annotations} />
      ) : (
        <div className="continuous-page-placeholder" aria-hidden="true" />
      )}
    </div>
  );
}

type Props = {
  pdf: PDFDocumentProxy;
  activePage: number;
  scale: number;
  annotations: Annotation[];
  jumpTarget: { pageIndex: number; sequence: number } | null;
  onActivePageChange: (pageIndex: number) => void;
  onWheel: (event: WheelEvent<HTMLDivElement>) => void;
};

export function ContinuousPdfView({
  pdf,
  activePage,
  scale,
  annotations,
  jumpTarget,
  onActivePageChange,
  onWheel,
}: Props) {
  const scrollArea = useRef<HTMLDivElement>(null);
  const initialPage = useRef(activePage);
  const frameRequest = useRef<number | null>(null);

  function scrollToPage(pageIndex: number, behavior: ScrollBehavior) {
    const page = scrollArea.current?.querySelector<HTMLElement>(`[data-page-index="${pageIndex}"]`);
    page?.scrollIntoView({ block: "start", behavior });
  }

  useEffect(() => {
    scrollToPage(initialPage.current, "instant");
  }, []);

  useEffect(() => {
    if (jumpTarget) scrollToPage(jumpTarget.pageIndex, "smooth");
  }, [jumpTarget]);

  function updateVisiblePage() {
    if (frameRequest.current !== null) return;
    frameRequest.current = requestAnimationFrame(() => {
      frameRequest.current = null;
      const area = scrollArea.current;
      if (!area) return;
      const targetY = area.getBoundingClientRect().top + Math.min(120, area.clientHeight * 0.25);
      let closestPage = 0;
      let closestDistance = Number.POSITIVE_INFINITY;
      area.querySelectorAll<HTMLElement>("[data-page-index]").forEach((page) => {
        const rect = page.getBoundingClientRect();
        const distance = targetY < rect.top
          ? rect.top - targetY
          : targetY > rect.bottom
            ? targetY - rect.bottom
            : 0;
        if (distance < closestDistance) {
          closestDistance = distance;
          closestPage = Number(page.dataset.pageIndex);
        }
      });
      if (closestPage !== activePage) onActivePageChange(closestPage);
    });
  }

  useEffect(() => () => {
    if (frameRequest.current !== null) cancelAnimationFrame(frameRequest.current);
  }, []);

  return (
    <div
      ref={scrollArea}
      className="continuous-pdf-scroll"
      onScroll={updateVisiblePage}
      onWheel={onWheel}
      tabIndex={0}
      aria-label={`PDF 连续滚动，共 ${pdf.numPages} 页`}
    >
      <div className="continuous-page-list">
        {Array.from({ length: pdf.numPages }, (_, pageIndex) => (
          <LazyPage
            key={pageIndex}
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
