export type RagProvider = "openai" | "qwen" | "custom";

export type RagPreferences = {
  provider: RagProvider;
  baseUrl: string;
  model: string;
  chunkChars: number;
  topK: number;
};

export type RagSettingsStatus = {
  preferences: RagPreferences;
  keyConfigured: boolean;
  configuredProviders: RagProvider[];
};

export type RagIndexStatus = {
  indexed: boolean;
  chunkCount: number;
  provider: string | null;
  model: string | null;
  updatedAt: string | null;
};

export type RagCitation = {
  number: number;
  chunkId: string;
  headingPath: string;
  excerpt: string;
  score: number;
};

export type RagMessage = {
  id: string;
  role: "user" | "assistant";
  content: string;
  citations: RagCitation[];
  createdAt: string;
};

export type RagAnswer = { answer: string; citations: RagCitation[] };
