import { cp, mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";

const root = new URL("../", import.meta.url);
for (const folder of ["cmaps", "standard_fonts", "wasm"]) {
  const destination = new URL(`public/pdfjs/${folder}/`, root);
  await mkdir(destination, { recursive: true });
  await cp(fileURLToPath(new URL(`node_modules/pdfjs-dist/${folder}/`, root)), fileURLToPath(destination), { recursive: true });
}
