import { invoke } from "@tauri-apps/api/core";
import type { PaperGroup, TodoItem } from "../contracts/library";

export const listPaperGroups = () => invoke<PaperGroup[]>("list_paper_groups");
export const createPaperGroup = (name: string) => invoke<PaperGroup>("create_paper_group", { name });
export const renamePaperGroup = (id: string, name: string) => invoke<void>("rename_paper_group", { id, name });
export const deletePaperGroup = (id: string) => invoke<void>("delete_paper_group", { id });
export const assignPaperGroup = (paperId: string, groupId: string | null) =>
  invoke<void>("assign_paper_group", { paperId, groupId });

export const listTodos = () => invoke<TodoItem[]>("list_todos");
export const createTodo = (title: string) => invoke<TodoItem>("create_todo", { title });
export const setTodoCompleted = (id: string, completed: boolean) =>
  invoke<void>("set_todo_completed", { id, completed });
export const deleteTodo = (id: string) => invoke<void>("delete_todo", { id });
