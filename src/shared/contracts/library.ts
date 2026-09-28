// IPC DTOs matching commands/library.rs. Generate these once tooling is chosen.
export type Paper = {
  revision: number;
  id: string;
  title: string;
  authors: string[];
  year: number | null;
  doi: string | null;
  sourceUrl: string | null;
  createdAt: string;
};

export type ImportResult = { paperId: string; duplicate: boolean };

export type MetadataInput = Omit<Paper, "revision" | "createdAt"> & { expectedRevision: number };
