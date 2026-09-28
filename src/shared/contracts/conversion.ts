export type MinerUPreferences = {
  model: "vlm" | "pipeline";
  language: "en" | "ch" | "ja" | "ko" | "fr" | "de" | "es";
  enableOcr: boolean;
  enableFormula: boolean;
  enableTable: boolean;
};

export type MinerUSettingsStatus = {
  preferences: MinerUPreferences;
  tokenConfigured: boolean;
};

export type MarkdownDocument = {
  documentId: string;
  relativePath: string;
  markdown: string;
  processor: string;
  model: string;
  updatedAt: string;
  assetBasePath: string;
};
