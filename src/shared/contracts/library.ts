// IPC DTOs matching commands/library.rs. Generate these once tooling is chosen.
export type Paper = {
  revision: number;
  id: string;
  title: string;
  authors: string[];
  year: number | null;
  doi: string | null;
  sourceUrl: string | null;
  venue: string | null;
  publisher: string | null;
  groupId: string | null;
  createdAt: string;
  originalName: string;
  hasMarkdown: boolean;
  hasTranslation: boolean;
  hasPaperTree: boolean;
  hasRag: boolean;
};

export type ImportResult = { paperId: string; duplicate: boolean };

export type MetadataInput = Omit<Paper, "revision" | "createdAt" | "originalName" | "groupId" | "hasMarkdown" | "hasTranslation" | "hasPaperTree" | "hasRag"> & { expectedRevision: number };

export type PaperGroup = { id: string; name: string; paperCount: number };
export type TodoItem = { id: string; title: string; completed: boolean; createdAt: string };
