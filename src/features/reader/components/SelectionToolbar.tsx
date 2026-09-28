export type TextSelection = {
  pageIndex: number;
  text: string;
  rects: Array<{ x: number; y: number; width: number; height: number }>;
  viewportRects: Array<{ left: number; top: number; width: number; height: number }>;
  position: { x: number; y: number };
};

type Props = {
  selection: TextSelection;
  busy: boolean;
  onHighlight: () => void;
  onNote: () => void;
  onTranslate: () => void;
  onCopy: () => void;
};

export function SelectionToolbar({ selection, busy, onHighlight, onNote, onTranslate, onCopy }: Props) {
  return (
    <div
      className="selection-toolbar"
      data-selection-ui
      role="toolbar"
      aria-label="所选文字操作"
      style={{ left: selection.position.x, top: selection.position.y }}
    >
      <span className="selection-page">第 {selection.pageIndex + 1} 页</span>
      <button disabled={busy} onClick={onHighlight}>高亮</button>
      <button disabled={busy} onClick={onNote}>批注</button>
      <button disabled={busy} onClick={onTranslate}>翻译</button>
      <button disabled={busy} onClick={onCopy}>复制</button>
    </div>
  );
}
