#!/usr/bin/env node
/**
 * 发布数据静态守卫（db/README.md §7.3 第 2 层）。
 *
 * 打包前校验「构建输入里没有测试 / 临时数据」，与 `cargo test --test
 * release_data_guard`（运行时基线）互补：本脚本管构建输入，后者管首次启动后的库。
 *
 * 检查项：
 * 1. `tauri.conf.json` 未配置 bundle.resources / externalBin / assets（否则会携带预置库）；
 * 2. 源码侧无数据库 / 备份 / 损坏隔离文件；`dist/` 命中则自动删除并报告（可重建产物）；
 * 3. 迁移 SQL 无测试标记，且 INSERT / UPDATE 只允许写系统基础数据表（白名单）；
 * 4. `dist/` 不含明文凭据（M6 / M12 的已登记生产 Key 只允许存在于 Rust 侧 SQL）；
 * 5. README §7.2 基线表与 `release_data_guard.rs` 的 BASELINE_ROWS 一致，防止定义漂移。
 *
 * 用法：`pnpm verify:release-data`（无第三方依赖，Windows / Linux 均可跑）。
 * 退出码：0 = 干净，1 = 不合规（CI 据此阻断发布）。
 */
import { existsSync, readFileSync, readdirSync, rmSync } from "node:fs";
import { dirname, extname, join, relative, resolve } from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const errors = [];
const notices = [];

/** 相对仓库根的绝对路径。 */
function abs(...parts) {
  return join(ROOT, ...parts);
}

function fail(message) {
  errors.push(message);
}

function note(message) {
  notices.push(message);
}

/** 递归遍历目录下所有文件；skip 为要跳过的目录名集合。 */
function walk(dir, visit, skip) {
  if (!existsSync(dir)) return;
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (skip.has(entry.name)) continue;
    const full = join(dir, entry.name);
    if (entry.isDirectory()) walk(full, visit, skip);
    else if (entry.isFile()) visit(full);
  }
}

// ---------- 1. bundle 配置：禁止把预置数据打进安装包 ----------

const conf = JSON.parse(readFileSync(abs("src-tauri", "tauri.conf.json"), "utf8"));
const bundle = conf.bundle ?? {};
for (const key of ["resources", "externalBin", "assets"]) {
  const value = bundle[key];
  if (value === undefined || value === null || value === false) continue;
  if (Array.isArray(value) && value.length === 0) continue;
  if (
    typeof value === "object" &&
    !Array.isArray(value) &&
    Object.keys(value).length === 0
  ) {
    continue;
  }
  fail(
    `tauri.conf.json 的 bundle.${key} 非空；它会把文件带进安装包，` +
      `发布构建必须只含编译产物与图标（见 db/README.md §7.1）`,
  );
}

// ---------- 2. 数据文件扫描 ----------

const DATA_FILE = /\.(db|db-journal|db-shm|db-wal|sqlite|sqlite3|bak|corrupt-[\w.-]+)$/i;
const SKIP_DIRS = new Set([
  "node_modules",
  "target",
  "gen",
  ".git",
  "dist",
  ".codebuddy",
]);

// 源码侧：出现即失败（源头必须干净，不能靠构建期静默删除掩盖）。
walk(
  abs("."),
  (file) => {
    if (!DATA_FILE.test(file)) return;
    fail(`源码侧存在数据文件 ${relative(ROOT, file)}；请移除后再打包`);
  },
  SKIP_DIRS,
);

// 构建产物侧：自动排除并报告（dist 由 pnpm build 生成、已被 .gitignore 忽略）。
if (!existsSync(abs("dist"))) {
  note("dist/ 不存在，跳过产物扫描（CI 中请先执行 pnpm build）");
} else {
  const removed = [];
  walk(
    abs("dist"),
    (file) => {
      if (!DATA_FILE.test(file)) return;
      rmSync(file, { force: true });
      removed.push(relative(ROOT, file));
    },
    SKIP_DIRS,
  );
  if (removed.length > 0) {
    note(`已从 dist/ 自动排除 ${removed.length} 个数据文件: ${removed.join(", ")}`);
  }
}

// ---------- 3. 迁移 SQL：测试标记 + 写表白名单 ----------

const FORBIDDEN_MARKERS = ["sk-old", "old/v1", "TEST_", "test-fixture", "mock-data"];
// 种子表：允许 INSERT 幂等种子（见 db/README.md §7.2 的 6 行系统基础数据）。
const INSERT_ALLOW = new Set([
  "pet_states",
  "app_settings",
  "default_model_settings",
  "search_settings",
  // M12：内置百度翻译凭据种子（单行 id=1，NOT EXISTS 守卫）
  "translate_settings",
  // M13：内置翻译服务商（百度 + 讯飞）凭据与各自月度上限，两家各一行
  "builtin_translate_providers",
]);
// 存量回填表：只允许 INSERT...SELECT 从已有业务数据聚合，禁止 VALUES 硬编码。
// M10 的平台月表回填属此类——对全新空库不产生任何行，不是种子数据。
const BACKFILL_INSERT_ALLOW = new Set(["quota_usage_platform_monthly"]);
// 数据迁移白名单：app_settings 结构升级（M3/M4/M7）+ quota 维度/额度对齐（M10）
// + M12 存量库补齐内置翻译凭据（条件 UPDATE，仅填补空值）。
const UPDATE_ALLOW = new Set([
  "app_settings",
  "quota_usage",
  "quota_usage_monthly",
  "translate_settings",
]);
const migrationsDir = abs("src-tauri", "src", "db", "migrations");
const sqlFiles = readdirSync(migrationsDir).filter((name) => name.endsWith(".sql"));

for (const name of sqlFiles) {
  const raw = readFileSync(join(migrationsDir, name), "utf8");
  // 去掉整行注释，避免把说明文字里的示例误判为真实数据。
  const code = raw
    .split("\n")
    .filter((line) => !line.trim().startsWith("--"))
    .join("\n");

  for (const marker of FORBIDDEN_MARKERS) {
    if (code.includes(marker)) {
      fail(`迁移 ${name} 含禁止标记 \`${marker}\`（测试 / 临时数据）`);
    }
  }
  for (const match of code.matchAll(/\bINSERT\s+INTO\s+([A-Za-z_][\w]*)/gi)) {
    const table = match[1];
    if (INSERT_ALLOW.has(table)) continue;
    if (BACKFILL_INSERT_ALLOW.has(table)) {
      // 取该 INSERT 起至下一个分号的语句体，要求 SELECT 派生且无 VALUES 常量。
      const tail = code.slice(match.index).split(";")[0];
      if (!/\bSELECT\b/i.test(tail) || /\bVALUES\s*\(/i.test(tail)) {
        fail(
          `迁移 ${name} 向回填表 \`${table}\` 写入非派生数据；` +
            `回填只允许 INSERT...SELECT（见 db/README.md §7.2）`,
        );
      }
      continue;
    }
    fail(
      `迁移 ${name} 向运行时表 \`${table}\` INSERT 种子；` +
        `只允许写 ${[...INSERT_ALLOW].join(" / ")}（见 db/README.md §7.2）`,
    );
  }
  // 只匹配语句开头的 UPDATE，跳过 `ON CONFLICT DO UPDATE SET` 这类子句
  // （它并非独立数据迁移语句，且会误把 SET 当成表名）。
  for (const match of code.matchAll(/(?:^|;|\n)\s*UPDATE\s+([A-Za-z_][\w]*)/gi)) {
    if (!UPDATE_ALLOW.has(match[1])) {
      fail(`迁移 ${name} UPDATE 了非白名单表 \`${match[1]}\``);
    }
  }
}
note(`已扫描 ${sqlFiles.length} 个迁移 SQL`);

// ---------- 4. dist/ 不得含明文凭据 ----------

// M6 的 Key 属已登记的生产凭据，只允许存在于 Rust 侧 SQL，不得进入前端产物。
const CREDENTIAL = /\b(sk-[A-Za-z0-9]{24,}|[a-f0-9]{32}\.[A-Za-z0-9]{16,})\b/i;
const TEXT_EXT = new Set([
  ".js",
  ".mjs",
  ".css",
  ".html",
  ".json",
  ".map",
  ".svg",
  ".txt",
  ".xml",
]);
if (existsSync(abs("dist"))) {
  walk(
    abs("dist"),
    (file) => {
      if (!TEXT_EXT.has(extname(file).toLowerCase())) return;
      if (CREDENTIAL.test(readFileSync(file, "utf8"))) {
        fail(`前端产物 ${relative(ROOT, file)} 含明文凭据`);
      }
    },
    SKIP_DIRS,
  );
}

// ---------- 5. 基线定义一致性（README ↔ Rust 守卫） ----------

const readme = readFileSync(abs("src-tauri", "src", "db", "README.md"), "utf8");
const section = readme.split("### 7.2 生产基线")[1]?.split("### 7.3")[0] ?? "";
const baselineFromReadme = new Map();
for (const match of section.matchAll(/\|\s*`([a-z_]+)`\s*\|\s*(\d+)\s*\|/g)) {
  baselineFromReadme.set(match[1], Number(match[2]));
}

const guardSource = readFileSync(
  abs("src-tauri", "tests", "release_data_guard.rs"),
  "utf8",
);
const baselineFromGuard = new Map();
for (const match of guardSource.matchAll(
  /\("([a-z_]+)",\s*"SELECT COUNT\(\*\) FROM [a-z_]+",\s*(\d+)\)/g,
)) {
  baselineFromGuard.set(match[1], Number(match[2]));
}

if (baselineFromReadme.size === 0 || baselineFromGuard.size === 0) {
  fail(
    "无法解析发布基线：README §7.2 表格或 release_data_guard.rs 的 BASELINE_ROWS 格式已变",
  );
} else {
  for (const [table, rows] of baselineFromReadme) {
    if (!baselineFromGuard.has(table)) {
      fail(`基线表 \`${table}\` 未写入 release_data_guard.rs 的 BASELINE_ROWS`);
    } else if (baselineFromGuard.get(table) !== rows) {
      fail(
        `基线表 \`${table}\` 行数定义不一致（README ${rows} / 守卫 ${baselineFromGuard.get(table)}）`,
      );
    }
  }
  for (const table of baselineFromGuard.keys()) {
    if (!baselineFromReadme.has(table)) {
      fail(`守卫中的 \`${table}\` 未登记到 README §7.2`);
    }
  }
  note(`基线一致：${baselineFromReadme.size} 张表`);
}

// ---------- 汇总 ----------

for (const message of notices) {
  process.stdout.write(`[verify-release-data] ${message}\n`);
}
if (errors.length > 0) {
  for (const message of errors) {
    process.stderr.write(`[verify-release-data] FAIL: ${message}\n`);
  }
  process.stderr.write(`[verify-release-data] 共 ${errors.length} 项不合规，阻断发布\n`);
  process.exitCode = 1;
} else {
  process.stdout.write("[verify-release-data] OK: 构建输入仅含系统基础数据\n");
}
