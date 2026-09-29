import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { PointerEvent as ReactPointerEvent, WheelEvent as ReactWheelEvent } from "react";
import type { Paper } from "../../shared/contracts/library";
import type { PaperTree, PaperTreeNode } from "../../shared/contracts/paperTree";
import { generatePaperTree, getPaperTree, savePaperTree } from "../../shared/ipc/paperTree";
import { TranslationSettings } from "../reader/components/TranslationSettings";

type Props = {
  paper: Paper;
  onSaved: () => void;
  onClose: () => void;
};

type Mode = "outline" | "tree";

const ROOT_WIDTH = 260;
const ROOT_HEIGHT = 100;
const NODE_WIDTH = 250;
const NODE_HEIGHT = 112;
const COLUMN_GAP = 110;
const ROW_GAP = 28;
const CANVAS_PADDING = 70;
const MIN_ZOOM = 0.3;
const MAX_ZOOM = 2.5;

type PositionedNode = { node: PaperTreeNode; x: number; y: number };
type TreeEdge = { id: string; fromX: number; fromY: number; toX: number; toY: number };
type TreeLayout = {
  root: { x: number; y: number };
  nodes: PositionedNode[];
  edges: TreeEdge[];
  width: number;
  height: number;
};

function buildTreeLayout(nodes: PaperTreeNode[], collapsed: Set<string>): TreeLayout {
  function span(node: PaperTreeNode): number {
    const children = collapsed.has(node.id) ? [] : node.children;
    if (children.length === 0) return NODE_HEIGHT;
    return Math.max(NODE_HEIGHT, children.reduce((total, child) => total + span(child), 0)
      + ROW_GAP * (children.length - 1));
  }

  const spans = nodes.map(span);
  const forestHeight = spans.reduce((total, value) => total + value, 0)
    + ROW_GAP * Math.max(0, nodes.length - 1);
  const height = Math.max(ROOT_HEIGHT, forestHeight) + CANVAS_PADDING * 2;
  const root = { x: CANVAS_PADDING, y: height / 2 - ROOT_HEIGHT / 2 };
  const positioned: PositionedNode[] = [];
  const edges: TreeEdge[] = [];
  let maxDepth = -1;

  function place(node: PaperTreeNode, depth: number, top: number, nodeSpan: number,
    parentX: number, parentY: number) {
    maxDepth = Math.max(maxDepth, depth);
    const x = CANVAS_PADDING + ROOT_WIDTH + COLUMN_GAP + depth * (NODE_WIDTH + COLUMN_GAP);
    const centerY = top + nodeSpan / 2;
    const y = centerY - NODE_HEIGHT / 2;
    positioned.push({ node, x, y });
    edges.push({ id: `${node.id}:${depth}`, fromX: parentX, fromY: parentY, toX: x, toY: centerY });

    if (collapsed.has(node.id) || node.children.length === 0) return;
    const childSpans = node.children.map(span);
    const childrenHeight = childSpans.reduce((total, value) => total + value, 0)
      + ROW_GAP * (node.children.length - 1);
    let childTop = centerY - childrenHeight / 2;
    node.children.forEach((child, index) => {
      place(child, depth + 1, childTop, childSpans[index], x + NODE_WIDTH, centerY);
      childTop += childSpans[index] + ROW_GAP;
    });
  }

  let top = (height - forestHeight) / 2;
  nodes.forEach((node, index) => {
    place(node, 0, top, spans[index], root.x + ROOT_WIDTH, height / 2);
    top += spans[index] + ROW_GAP;
  });
  const columns = maxDepth + 1;
  const width = CANVAS_PADDING * 2 + ROOT_WIDTH
    + (columns > 0 ? COLUMN_GAP + columns * NODE_WIDTH + Math.max(0, columns - 1) * COLUMN_GAP : 0);
  return { root, nodes: positioned, edges, width, height };
}

function freshNode(title = "新节点"): PaperTreeNode {
  return { id: crypto.randomUUID(), title, note: "", children: [] };
}

function updateNode(nodes: PaperTreeNode[], id: string, update: (node: PaperTreeNode) => PaperTreeNode): PaperTreeNode[] {
  return nodes.map((node) => node.id === id
    ? update(node)
    : { ...node, children: updateNode(node.children, id, update) });
}

function removeNode(nodes: PaperTreeNode[], id: string): PaperTreeNode[] {
  return nodes
    .filter((node) => node.id !== id)
    .map((node) => ({ ...node, children: removeNode(node.children, id) }));
}

function addSibling(nodes: PaperTreeNode[], id: string): [PaperTreeNode[], boolean] {
  const index = nodes.findIndex((node) => node.id === id);
  if (index >= 0) {
    const next = [...nodes];
    next.splice(index + 1, 0, freshNode());
    return [next, true];
  }
  for (let i = 0; i < nodes.length; i += 1) {
    const [children, added] = addSibling(nodes[i].children, id);
    if (added) {
      const next = [...nodes];
      next[i] = { ...nodes[i], children };
      return [next, true];
    }
  }
  return [nodes, false];
}

function countNodes(nodes: PaperTreeNode[]): number {
  return nodes.reduce((total, node) => total + 1 + countNodes(node.children), 0);
}

type OutlineNodeProps = {
  node: PaperTreeNode;
  depth: number;
  disabled: boolean;
  onChange: (id: string, field: "title" | "note", value: string) => void;
  onAddChild: (id: string) => void;
  onAddSibling: (id: string) => void;
  onRemove: (id: string) => void;
};

function OutlineNode({ node, depth, disabled, onChange, onAddChild, onAddSibling, onRemove }: OutlineNodeProps) {
  return (
    <li className="paper-tree-outline-node">
      <div className="paper-tree-outline-row">
        <span className="paper-tree-bullet" aria-hidden="true" />
        <div className="paper-tree-node-fields">
          <input aria-label={`第 ${depth} 层节点标题`} value={node.title} disabled={disabled}
            onChange={(event) => onChange(node.id, "title", event.target.value)} />
          <textarea aria-label={`${node.title || "节点"}说明`} rows={node.note ? 2 : 1} value={node.note}
            disabled={disabled} placeholder="补充论点、证据或你的理解…"
            onChange={(event) => onChange(node.id, "note", event.target.value)} />
        </div>
        <div className="paper-tree-node-actions">
          <button disabled={disabled || depth >= 12} onClick={() => onAddChild(node.id)} title="添加子节点">＋子级</button>
          <button disabled={disabled} onClick={() => onAddSibling(node.id)} title="添加同级节点">＋同级</button>
          <button className="danger-link" disabled={disabled} onClick={() => onRemove(node.id)} title="删除节点">删除</button>
        </div>
      </div>
      {node.children.length > 0 && (
        <ul className="paper-tree-outline-children">
          {node.children.map((child) => (
            <OutlineNode key={child.id} node={child} depth={depth + 1} disabled={disabled}
              onChange={onChange} onAddChild={onAddChild} onAddSibling={onAddSibling} onRemove={onRemove} />
          ))}
        </ul>
      )}
    </li>
  );
}

export function PaperTreeWorkspace({ paper, onSaved, onClose }: Props) {
  const [tree, setTree] = useState<PaperTree | null>(null);
  const [mode, setMode] = useState<Mode>("outline");
  const [dirty, setDirty] = useState(false);
  const [saving, setSaving] = useState(false);
  const [generating, setGenerating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [generationStatus, setGenerationStatus] = useState<string | null>(null);
  const [showModelSettings, setShowModelSettings] = useState(false);
  const [collapsed, setCollapsed] = useState<Set<string>>(() => new Set());
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const zoomRef = useRef(1);
  const canvasRef = useRef<HTMLElement | null>(null);
  const dragRef = useRef<{ pointerId: number; x: number; y: number; panX: number; panY: number } | null>(null);

  useEffect(() => {
    let active = true;
    getPaperTree(paper.id, paper.title)
      .then((value) => { if (active) setTree(value); })
      .catch((reason) => { if (active) setError(`Paper Tree 加载失败：${String(reason)}`); });
    return () => { active = false; };
  }, [paper.id, paper.title]);

  const totalNodes = useMemo(() => tree ? countNodes(tree.nodes) : 0, [tree]);
  const treeLayout = useMemo(() => buildTreeLayout(tree?.nodes ?? [], collapsed), [tree?.nodes, collapsed]);

  const fitTree = useCallback(() => {
    const viewport = canvasRef.current?.getBoundingClientRect();
    if (!viewport) return;
    const nextZoom = Math.min(1, Math.max(MIN_ZOOM,
      Math.min((viewport.width - 56) / treeLayout.width, (viewport.height - 56) / treeLayout.height)));
    zoomRef.current = nextZoom;
    setZoom(nextZoom);
    setPan({
      x: (viewport.width - treeLayout.width * nextZoom) / 2,
      y: (viewport.height - treeLayout.height * nextZoom) / 2,
    });
  }, [treeLayout.height, treeLayout.width]);

  useEffect(() => {
    if (mode !== "tree" || !tree) return;
    const frame = requestAnimationFrame(fitTree);
    return () => cancelAnimationFrame(frame);
  }, [mode, tree?.paperId, collapsed, fitTree]);

  function zoomAt(nextZoom: number, clientX?: number, clientY?: number) {
    const viewport = canvasRef.current?.getBoundingClientRect();
    const clamped = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, nextZoom));
    const previousZoom = zoomRef.current;
    zoomRef.current = clamped;
    if (!viewport) { setZoom(clamped); return; }
    const anchorX = clientX === undefined ? viewport.width / 2 : clientX - viewport.left;
    const anchorY = clientY === undefined ? viewport.height / 2 : clientY - viewport.top;
    setPan((current) => ({
      x: anchorX - (anchorX - current.x) * clamped / previousZoom,
      y: anchorY - (anchorY - current.y) * clamped / previousZoom,
    }));
    setZoom(clamped);
  }

  useEffect(() => {
    if (mode !== "tree") return;
    function handleShortcut(event: KeyboardEvent) {
      if (!(event.metaKey || event.ctrlKey)) return;
      if (event.key === "+" || event.key === "=") {
        event.preventDefault();
        zoomAt(zoomRef.current * 1.2);
      } else if (event.key === "-") {
        event.preventDefault();
        zoomAt(zoomRef.current / 1.2);
      } else if (event.key === "0") {
        event.preventDefault();
        fitTree();
      }
    }
    window.addEventListener("keydown", handleShortcut);
    return () => window.removeEventListener("keydown", handleShortcut);
  }, [mode, fitTree]);

  function handleCanvasWheel(event: ReactWheelEvent<HTMLElement>) {
    event.preventDefault();
    if (event.metaKey || event.ctrlKey) {
      zoomAt(zoomRef.current * Math.exp(-event.deltaY * 0.002), event.clientX, event.clientY);
      return;
    }
    setPan((current) => ({ x: current.x - event.deltaX, y: current.y - event.deltaY }));
  }

  function handlePointerDown(event: ReactPointerEvent<SVGSVGElement>) {
    if (event.button !== 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    dragRef.current = { pointerId: event.pointerId, x: event.clientX, y: event.clientY, panX: pan.x, panY: pan.y };
  }

  function handlePointerMove(event: ReactPointerEvent<SVGSVGElement>) {
    const drag = dragRef.current;
    if (!drag || drag.pointerId !== event.pointerId) return;
    setPan({ x: drag.panX + event.clientX - drag.x, y: drag.panY + event.clientY - drag.y });
  }

  function handlePointerUp(event: ReactPointerEvent<SVGSVGElement>) {
    if (dragRef.current?.pointerId === event.pointerId) dragRef.current = null;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
  }

  function changeNodes(updater: (nodes: PaperTreeNode[]) => PaperTreeNode[]) {
    setTree((current) => current ? { ...current, nodes: updater(current.nodes) } : current);
    setDirty(true);
  }

  function handleChange(id: string, field: "title" | "note", value: string) {
    changeNodes((nodes) => updateNode(nodes, id, (node) => ({ ...node, [field]: value })));
  }

  async function handleSave() {
    if (!tree || saving) return;
    setSaving(true);
    setError(null);
    try {
      const saved = await savePaperTree({ paperId: tree.paperId, title: tree.title, nodes: tree.nodes });
      setTree(saved);
      setDirty(false);
      onSaved();
    } catch (reason) {
      setError(`保存失败：${String(reason)}`);
    } finally {
      setSaving(false);
    }
  }

  async function handleGenerate() {
    if (generating || saving) return;
    if ((tree?.saved || dirty) && !window.confirm("AI 生成会替换当前 Paper Tree。确定继续吗？")) return;
    setGenerating(true);
    setError(null);
    setGenerationStatus("正在读取 Markdown 并分析论文；长文可能需要多次模型调用…");
    try {
      const result = await generatePaperTree(paper.id);
      setTree(result.tree);
      setDirty(false);
      setMode("tree");
      onSaved();
      setGenerationStatus(
        `${result.provider} / ${result.model} 已生成并保存${result.sourceTruncated ? "；原文较长，本次从全文均匀抽样分析了 144,000 个字符" : ""}。`,
      );
    } catch (reason) {
      setError(`AI 生成失败：${String(reason)}`);
      setGenerationStatus(null);
    } finally {
      setGenerating(false);
    }
  }

  return (
    <div className="paper-tree-workspace">
      <header className="paper-tree-heading">
        <button className="secondary-button" onClick={onClose}>返回文献库</button>
        <div className="paper-tree-heading-copy">
          <h2>Paper Tree</h2>
          <p>{paper.title}</p>
        </div>
        <div className="paper-tree-mode" aria-label="Paper Tree 视图模式">
          <button aria-pressed={mode === "outline"} onClick={() => setMode("outline")}>大纲编辑</button>
          <button aria-pressed={mode === "tree"} onClick={() => setMode("tree")}>树状视图</button>
        </div>
        <button className="secondary-button" disabled={generating || saving}
          onClick={() => setShowModelSettings(true)}>模型设置</button>
        <button className="paper-tree-ai-button" disabled={!tree || generating || saving}
          onClick={() => void handleGenerate()}>{generating ? "AI 分析中…" : "AI 生成并保存"}</button>
        <span className="paper-tree-save-state">{dirty ? "有未保存修改" : tree?.saved ? "已保存" : "模板未保存"}</span>
        <button className="primary-button" disabled={!tree || saving || generating} onClick={handleSave}>
          {saving ? "保存中…" : "保存 Paper Tree"}
        </button>
      </header>

      {error && <p className="paper-tree-error" role="alert">{error}</p>}
      {generationStatus && <p className="paper-tree-generation-status" role="status">{generationStatus}</p>}
      {!tree && !error && <p className="reader-notice" role="status">正在加载 Paper Tree…</p>}

      {tree && mode === "outline" && (
        <main className="paper-tree-outline-scroll">
          <section className="paper-tree-outline-paper">
            <div className="paper-tree-title-row">
              <div>
                <span className="paper-tree-eyebrow">论文结构 · {totalNodes} 个节点</span>
                <input className="paper-tree-title-input" aria-label="Paper Tree 标题" value={tree.title}
                  disabled={saving} onChange={(event) => {
                    setTree({ ...tree, title: event.target.value });
                    setDirty(true);
                  }} />
              </div>
              <button className="secondary-button" disabled={saving}
                onClick={() => changeNodes((nodes) => [...nodes, freshNode("新章节")])}>＋ 添加顶层节点</button>
            </div>
            <p className="paper-tree-help">按模板逐层填写论文的任务、动机、方法、实验和局限。每个节点都可以添加同级或子级内容。</p>
            <ul className="paper-tree-outline-list">
              {tree.nodes.map((node) => (
                <OutlineNode key={node.id} node={node} depth={1} disabled={saving}
                  onChange={handleChange}
                  onAddChild={(id) => changeNodes((nodes) => updateNode(nodes, id, (item) => ({
                    ...item, children: [...item.children, freshNode()],
                  })))}
                  onAddSibling={(id) => changeNodes((nodes) => addSibling(nodes, id)[0])}
                  onRemove={(id) => changeNodes((nodes) => removeNode(nodes, id))} />
              ))}
            </ul>
          </section>
        </main>
      )}

      {tree && mode === "tree" && (
        <main ref={canvasRef} className="paper-tree-canvas" onWheel={handleCanvasWheel}>
          <div className="paper-tree-canvas-toolbar">
            <button aria-label="缩小" onClick={() => zoomAt(zoom / 1.2)}>−</button>
            <span>{Math.round(zoom * 100)}%</span>
            <button aria-label="放大" onClick={() => zoomAt(zoom * 1.2)}>＋</button>
            <button onClick={fitTree}>适应窗口</button>
          </div>
          <p className="paper-tree-canvas-help">滚轮/触控板移动 · 拖拽画布 · ⌘/Ctrl + 滚轮或 ＋/− 缩放 · ⌘/Ctrl + 0 适应</p>
          <svg className="paper-tree-viewport" aria-label={`${tree.title} 的树状结构`}
            onPointerDown={handlePointerDown} onPointerMove={handlePointerMove}
            onPointerUp={handlePointerUp} onPointerCancel={handlePointerUp}>
            <g transform={`translate(${pan.x} ${pan.y}) scale(${zoom})`}>
              <g className="paper-tree-edge-layer">
                {treeLayout.edges.map((edge) => {
                  const bend = edge.fromX + (edge.toX - edge.fromX) / 2;
                  return <path key={edge.id}
                    d={`M ${edge.fromX} ${edge.fromY} C ${bend} ${edge.fromY}, ${bend} ${edge.toY}, ${edge.toX} ${edge.toY}`} />;
                })}
              </g>
              <foreignObject x={treeLayout.root.x} y={treeLayout.root.y} width={ROOT_WIDTH} height={ROOT_HEIGHT}>
                <div className="paper-tree-svg-root">
                  <span>Paper</span><strong>{tree.title || paper.title}</strong>
                </div>
              </foreignObject>
              {treeLayout.nodes.map(({ node, x, y }) => {
                const hidden = collapsed.has(node.id);
                return (
                  <foreignObject key={node.id} x={x} y={y} width={NODE_WIDTH} height={NODE_HEIGHT}>
                    <article className="paper-tree-svg-card">
                      <header>
                        <strong>{node.title || "未命名节点"}</strong>
                        {node.children.length > 0 && (
                          <button aria-label={hidden ? "展开子节点" : "收起子节点"}
                            onPointerDown={(event) => event.stopPropagation()}
                            onClick={() => setCollapsed((current) => {
                              const next = new Set(current);
                              if (next.has(node.id)) next.delete(node.id); else next.add(node.id);
                              return next;
                            })}>{hidden ? `＋${node.children.length}` : "−"}</button>
                        )}
                      </header>
                      {node.note && <p>{node.note}</p>}
                    </article>
                  </foreignObject>
                );
              })}
            </g>
          </svg>
        </main>
      )}

      {showModelSettings && (
        <aside className="paper-tree-settings" aria-label="Paper Tree 模型设置">
          <header><h3>模型设置</h3><button aria-label="关闭模型设置" onClick={() => setShowModelSettings(false)}>×</button></header>
          <p className="muted">Paper Tree 与翻译共用 OpenAI-compatible 服务和 API Key。生成前需先完成 PDF 到 Markdown 的转换。</p>
          <TranslationSettings />
        </aside>
      )}
    </div>
  );
}
