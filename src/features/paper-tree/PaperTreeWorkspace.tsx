import { useEffect, useMemo, useState } from "react";
import type { Paper } from "../../shared/contracts/library";
import type { PaperTree, PaperTreeNode } from "../../shared/contracts/paperTree";
import { getPaperTree, savePaperTree } from "../../shared/ipc/paperTree";

type Props = {
  paper: Paper;
  onSaved: () => void;
  onClose: () => void;
};

type Mode = "outline" | "tree";

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

function TreeBranch({ node, collapsed, onToggle }: {
  node: PaperTreeNode;
  collapsed: Set<string>;
  onToggle: (id: string) => void;
}) {
  const hidden = collapsed.has(node.id);
  return (
    <li className="paper-tree-branch">
      <article className="paper-tree-card">
        <header>
          <strong>{node.title || "未命名节点"}</strong>
          {node.children.length > 0 && (
            <button aria-label={hidden ? "展开子节点" : "收起子节点"} onClick={() => onToggle(node.id)}>
              {hidden ? `＋${node.children.length}` : "−"}
            </button>
          )}
        </header>
        {node.note && <p>{node.note}</p>}
      </article>
      {!hidden && node.children.length > 0 && (
        <ul className="paper-tree-branches">
          {node.children.map((child) => (
            <TreeBranch key={child.id} node={child} collapsed={collapsed} onToggle={onToggle} />
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
  const [error, setError] = useState<string | null>(null);
  const [collapsed, setCollapsed] = useState<Set<string>>(() => new Set());

  useEffect(() => {
    let active = true;
    getPaperTree(paper.id, paper.title)
      .then((value) => { if (active) setTree(value); })
      .catch((reason) => { if (active) setError(`Paper Tree 加载失败：${String(reason)}`); });
    return () => { active = false; };
  }, [paper.id, paper.title]);

  const totalNodes = useMemo(() => tree ? countNodes(tree.nodes) : 0, [tree]);

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
        <span className="paper-tree-save-state">{dirty ? "有未保存修改" : tree?.saved ? "已保存" : "模板未保存"}</span>
        <button className="primary-button" disabled={!tree || saving} onClick={handleSave}>
          {saving ? "保存中…" : "保存 Paper Tree"}
        </button>
      </header>

      {error && <p className="paper-tree-error" role="alert">{error}</p>}
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
        <main className="paper-tree-canvas-scroll">
          <div className="paper-tree-root-layout">
            <article className="paper-tree-root-card">
              <span>Paper</span>
              <strong>{tree.title || paper.title}</strong>
            </article>
            <ul className="paper-tree-branches paper-tree-root-branches">
              {tree.nodes.map((node) => (
                <TreeBranch key={node.id} node={node} collapsed={collapsed}
                  onToggle={(id) => setCollapsed((current) => {
                    const next = new Set(current);
                    if (next.has(id)) next.delete(id); else next.add(id);
                    return next;
                  })} />
              ))}
            </ul>
          </div>
        </main>
      )}
    </div>
  );
}
