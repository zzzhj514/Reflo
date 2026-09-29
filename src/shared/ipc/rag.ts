import { invoke } from "@tauri-apps/api/core";
import type { RagAnswer, RagIndexStatus, RagMessage, RagPreferences, RagSettingsStatus } from "../contracts/rag";

export const getRagSettings = () => invoke<RagSettingsStatus>("get_rag_settings");
export const saveRagSettings = (preferences: RagPreferences, apiKey?: string) =>
  invoke<RagSettingsStatus>("save_rag_settings", { settings: { preferences, apiKey: apiKey?.trim() || null } });
export const getRagIndexStatus = (paperId: string) =>
  invoke<RagIndexStatus>("get_rag_index_status", { paperId });
export const indexPaperRag = (paperId: string) =>
  invoke<RagIndexStatus>("index_paper_rag", { paperId });
export const listRagMessages = (paperId: string) =>
  invoke<RagMessage[]>("list_rag_messages", { paperId });
export const askPaperRag = (paperId: string, question: string) =>
  invoke<RagAnswer>("ask_paper_rag", { paperId, question });
