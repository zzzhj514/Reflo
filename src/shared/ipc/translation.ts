import { invoke } from "@tauri-apps/api/core";
import type { MarkdownTranslationDocument, TranslationPreferences, TranslationResult, TranslationSettingsStatus } from "../contracts/translation";

export const getTranslationSettings = () =>
  invoke<TranslationSettingsStatus>("get_translation_settings");

export const saveTranslationSettings = (preferences: TranslationPreferences, apiKey?: string) =>
  invoke<TranslationSettingsStatus>("save_translation_settings", {
    settings: { preferences, apiKey: apiKey?.trim() || null },
  });

export const translateText = (text: string) =>
  invoke<TranslationResult>("translate_text", { text });

export const getMarkdownTranslation = (paperId: string) =>
  invoke<MarkdownTranslationDocument | null>("get_markdown_translation", { paperId });

export const translateMarkdownDocument = (paperId: string) =>
  invoke<MarkdownTranslationDocument>("translate_markdown_document", { paperId });
