/**
 * 单向同步根仓库 latest.json 到 website/docs/public/latest.json。
 * 仅拷贝、只读、不回写、不改写语义。下载页通过 fetch('/latest.json') 展示版本号。
 * 注意：VitePress 的 publicDir 相对 srcDir 解析，srcDir 为 docs 时资源目录是 docs/public。
 */
import { copyFileSync, existsSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "..", "..");
const src = resolve(repoRoot, "latest.json");
const destDir = resolve(__dirname, "..", "docs", "public");
const dest = resolve(destDir, "latest.json");

if (!existsSync(src)) {
  console.error(
    "[sync-latest] 根目录 latest.json 不存在，跳过拷贝（下载页将显示默认值）。",
  );
  process.exit(0);
}

if (!existsSync(destDir)) {
  mkdirSync(destDir, { recursive: true });
}

copyFileSync(src, dest);
console.log(`[sync-latest] 已单向拷贝 ${src} -> ${dest}`);
