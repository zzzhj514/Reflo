export type TranslationProvider = "openai" | "deepseek" | "custom";
export type TranslationTargetLanguage = "auto" | "zh-CN" | "en" | "ja" | "ko" | "fr" | "de" | "es";

export type TranslationPreferences = {
  provider: TranslationProvider;
  baseUrl: string;
  model: string;
  targetLanguage: TranslationTargetLanguage;
};

export type TranslationSettingsStatus = {
  preferences: TranslationPreferences;
  keyConfigured: boolean;
  configuredProviders: TranslationProvider[];
};

export type TranslationResult = {
  sourceText: string;
  translatedText: string;
  targetLanguage: string;
  provider: TranslationProvider;
  model: string;
  truncated: boolean;
};

export type MarkdownTranslationDocument = {
  documentId: string;
  relativePath: string;
  markdown: string;
  provider: TranslationProvider;
  model: string;
  targetLanguage: string;
  updatedAt: string;
  assetBasePath: string;
};
