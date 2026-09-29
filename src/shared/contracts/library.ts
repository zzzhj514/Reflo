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
  createdAt: string;
  originalName: string;
  hasMarkdown: boolean;
  hasTranslation: boolean;
  hasPaperTree: boolean;
  hasRag: boolean;
};

export type ImportResult = { paperId: string; duplicate: boolean };

export type MetadataInput = Omit<Paper, "revision" | "createdAt" | "originalName" | "hasMarkdown" | "hasTranslation" | "hasPaperTree" | "hasRag"> & { expectedRevision: number };
