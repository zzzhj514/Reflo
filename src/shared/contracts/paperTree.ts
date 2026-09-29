export type PaperTreeNode = {
  id: string;
  title: string;
  note: string;
  children: PaperTreeNode[];
};

export type PaperTree = {
  paperId: string;
  title: string;
  nodes: PaperTreeNode[];
  updatedAt: string | null;
  saved: boolean;
};

export type SavePaperTreeInput = Pick<PaperTree, "paperId" | "title" | "nodes">;
