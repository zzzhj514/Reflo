export type AnnotationRect = {
  x: number;
  y: number;
  width: number;
  height: number;
};

export type AnnotationKind = "highlight" | "note";

export type Annotation = {
  id: string;
  documentId: string;
  pageIndex: number;
  kind: AnnotationKind;
  selectedText: string;
  note: string | null;
  rects: AnnotationRect[];
  color: string;
  createdAt: string;
};

export type NewAnnotation = Omit<Annotation, "id" | "createdAt">;
