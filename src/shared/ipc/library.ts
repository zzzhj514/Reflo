import { invoke } from "@tauri-apps/api/core";
import type { ImportResult, MetadataInput, Paper } from "../contracts/library";

export const listPapers = () => invoke<Paper[]>("list_papers");
export const importPdf = (sourcePath: string) =>
  invoke<ImportResult>("import_pdf", { sourcePath });

export const updateMetadata = (input: MetadataInput) =>
  invoke<Paper>("update_metadata", { input });
