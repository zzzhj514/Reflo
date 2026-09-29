import { useEffect, useState } from "react";
import type { FormEvent } from "react";
import type { TodoItem } from "../../shared/contracts/library";
import { createTodo, deleteTodo, listTodos, setTodoCompleted } from "../../shared/ipc/organization";

export function TodoDashboard() {
  const [items, setItems] = useState<TodoItem[]>([]);
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    listTodos().then(setItems).catch((cause) => setError(String(cause)));
  }, []);

  async function add(event: FormEvent) {
    event.preventDefault();
    const title = draft.trim();
    if (!title || busy) return;
    setBusy(true); setError(null);
    try {
      const item = await createTodo(title);
      setItems((current) => [item, ...current]);
      setDraft("");
    } catch (cause) { setError(String(cause)); }
    finally { setBusy(false); }
  }

  async function toggle(item: TodoItem) {
    setError(null);
    setItems((current) => current.map((value) => value.id === item.id ? { ...value, completed: !value.completed } : value));
    try { await setTodoCompleted(item.id, !item.completed); }
    catch (cause) {
      setItems((current) => current.map((value) => value.id === item.id ? item : value));
      setError(String(cause));
    }
  }

  async function remove(id: string) {
    const previous = items;
    setItems((current) => current.filter((item) => item.id !== id));
    try { await deleteTodo(id); }
    catch (cause) { setItems(previous); setError(String(cause)); }
  }

  const openCount = items.filter((item) => !item.completed).length;
  return (
    <section className="dashboard">
      <header className="dashboard-welcome">
        <span className="dashboard-eyebrow">REFLO WORKSPACE</span>
        <h2>今天准备读什么？</h2>
        <p>选择左侧论文开始阅读，或先记下接下来的研究任务。</p>
      </header>
      <div className="todo-card">
        <div className="todo-card-heading">
          <div><h3>待办事项</h3><p>{openCount} 项未完成</p></div>
          <span aria-hidden="true">✓</span>
        </div>
        <form className="todo-create" onSubmit={add}>
          <input maxLength={500} value={draft} placeholder="添加阅读、整理或写作任务…"
            onChange={(event) => setDraft(event.target.value)} />
          <button className="primary-button" disabled={busy || !draft.trim()}>添加</button>
        </form>
        {error && <p className="form-error" role="alert">{error}</p>}
        {items.length === 0 ? <div className="todo-empty"><span>○</span><p>还没有待办，先添加一项吧。</p></div> : (
          <ul className="todo-list">
            {items.map((item) => <li key={item.id} data-completed={item.completed || undefined}>
              <label><input type="checkbox" checked={item.completed} onChange={() => void toggle(item)} />
                <span>{item.title}</span></label>
              <button aria-label={`删除待办：${item.title}`} onClick={() => void remove(item.id)}>×</button>
            </li>)}
          </ul>
        )}
      </div>
    </section>
  );
}
