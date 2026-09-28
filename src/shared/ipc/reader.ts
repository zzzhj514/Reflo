import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import type { Annotation, NewAnnotation } from "../contracts/reader";

export type ReadingDocument = { documentId: string; path: string; pageIndex: number; scale: number };
export async function openDocument(paperId: string) {
  const document = await invoke<ReadingDocument>("open_document", { paperId });
  return { ...document, url: convertFileSrc(document.path) };
}

// Serialize writes so rapid page changes cannot persist out of order.
let pendingSave: Promise<unknown> = Promise.resolve();
export function saveReadingPosition(documentId: string, pageIndex: number, scale: number) {
  const result = pendingSave.catch(() => {}).then(() =>
    invoke<void>("save_reading_position", { documentId, pageIndex, scale }),
  );
  pendingSave = result;
  return result;
}

export function listAnnotations(documentId: string) {
  return invoke<Annotation[]>("list_annotations", { documentId });
}

export function createAnnotation(input: NewAnnotation) {
  return invoke<Annotation>("create_annotation", { input });
}

export function deleteAnnotation(id: string) {
  return invoke<void>("delete_annotation", { id });
}
