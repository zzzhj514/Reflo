import { invoke } from "@tauri-apps/api/core";
import type { MarkdownDocument, MinerUPreferences, MinerUSettingsStatus } from "../contracts/conversion";

export function getMineruSettings() {
  return invoke<MinerUSettingsStatus>("get_mineru_settings");
}

export function saveMineruSettings(preferences: MinerUPreferences, apiToken?: string) {
  return invoke<MinerUSettingsStatus>("save_mineru_settings", {
    settings: { preferences, apiToken: apiToken?.trim() || null },
  });
}

export function getMarkdownDocument(paperId: string) {
  return invoke<MarkdownDocument | null>("get_markdown_document", { paperId });
}

export function convertPdfToMarkdown(paperId: string) {
  return invoke<MarkdownDocument>("convert_pdf_to_markdown", { paperId });
}
