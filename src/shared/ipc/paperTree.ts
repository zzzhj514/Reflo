import { invoke } from "@tauri-apps/api/core";
import type { PaperTree, SavePaperTreeInput } from "../contracts/paperTree";

export const getPaperTree = (paperId: string, paperTitle: string) =>
  invoke<PaperTree>("get_paper_tree", { paperId, paperTitle });

export const savePaperTree = (input: SavePaperTreeInput) =>
  invoke<PaperTree>("save_paper_tree", { input });
