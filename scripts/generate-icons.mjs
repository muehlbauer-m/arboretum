#!/usr/bin/env node
/**
 * Rasterize src/assets/icon-master.svg to a 1024×1024 PNG, then run
 * `tauri icon` to regenerate the full platform icon set
 * (src-tauri/icons/*.png, icon.ico, icon.icns).
 *
 * Usage: `npm run generate:icons` (see package.json scripts).
 *
 * Dependencies (devDependency): sharp.
 */
import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, "..");

let sharp;
try {
  sharp = (await import("sharp")).default;
} catch {
  console.error(
    "\n[generate-icons] `sharp` is not installed.\n" +
      "Run `npm install -D sharp` first, then re-run this script.\n"
  );
  process.exit(1);
}

const svgPath = join(ROOT, "src/assets/icon-master.svg");
const outPng = join(ROOT, "icon-source.png");

console.log(`[generate-icons] Rasterizing ${svgPath} → ${outPng} (1024×1024)`);

const svg = readFileSync(svgPath);
const png = await sharp(svg).resize(1024, 1024).png().toBuffer();
writeFileSync(outPng, png);

console.log("[generate-icons] Running `tauri icon` to build all platform sets…");
try {
  execFileSync("npx", ["tauri", "icon", outPng], {
    stdio: "inherit",
    cwd: ROOT,
    shell: true,
  });
} catch (err) {
  console.error("[generate-icons] `tauri icon` failed:", err.message);
  process.exit(1);
}

console.log(
  "\n[generate-icons] Done. Regenerated icons live in src-tauri/icons/." +
    "\nReview the diff and commit if the output matches the design."
);
