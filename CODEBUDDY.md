# CODEBUDDY.md This file provides guidance to CodeBuddy when working with code in this repository.

## Project Overview

`七七` is a Windows desktop digital-pet app ("桌宠" — a desktop digital life that lives on your screen). It is built with **Tauri 2 (Rust + WebView2)** as the system shell and **Vue 3 + TypeScript + Vite** as the frontend, with **PIXI.js** for code-drawn pet rendering. The product spec lives in `docs/design/桌宠.md`, the dev/code-style rules in `docs/design/rule.md`, and the settings-window mockup in `docs/design/pet-design/index.html`.

**Current state vs. target:** The skeleton is rebuilt per `docs/design/项目目录结构规范.md` (v1.1): Rust domain modules + `commands/` thin layer + `AppError` are in place (`src-tauri/src/`), and the frontend follows the window-dispatched structure (`api/state/renderer/composables/components/views`). Dual windows are configured in `src-tauri/tauri.conf.json` (B1 done): window A is a transparent always-on-top 400×400 pet layer, window B a 1000×660 settings layer with a self-drawn title bar; close is intercepted as `hide` (process stays resident; exit goes through the tray, C2). `Cargo.toml` already pre-includes `rusqlite` (with `bundled-sqlcipher-vendored-openssl`) and `sqlite-vec`, signaling the intended encrypted-DB architecture. The design docs describe the full target product; treat them as the specification, and this file as the map — the gap is the backlog (B2/B4/B5/C*).

## Commands

All commands use `pnpm` (see `package.json`). A Rust toolchain and WebView2 are required for any `tauri` command.

- **`pnpm install`** — Install frontend deps from `pnpm-lock.yaml`. Run once after clone and after changing `package.json`. Does not build the Rust side (Cargo fetches on first `tauri` run).

- **`pnpm dev`** — Start only the Vite dev server on `http://localhost:1420` (fixed port, `vite.config.ts`). Use to iterate on Vue/PIXI UI without launching the desktop window.

- **`pnpm tauri dev`** — Build/run the full desktop app: launches the Rust binary (WebView2 window) and the Vite dev server together, with hot-reload. The primary command during feature development.

- **`pnpm build`** — Frontend-only production build: runs `vue-tsc --noEmit` (TypeScript type-check, strict mode) then `vite build` to `dist/`. Fails on any type error.

- **`pnpm tauri build`** — Produce the distributable (MSI/installer) using `tauri.conf.json` bundle settings. Slow first run (compiles all Rust deps, including OpenSSL/SQLCipher).

- **`pnpm preview`** — Serve the built `dist/` locally to verify the production frontend bundle.

- **`pnpm verify:release-data`** — Pre-release data guard (`scripts/verify-release-data.mjs`): asserts `bundle.resources` is empty, scans source/`dist/` for `*.db` and backup files (auto-removes hits in `dist/`), checks migration SQL for test markers and non-baseline INSERTs, and verifies README §7.2 matches `release_data_guard.rs`. Run before packaging; also enforced in `release.yml`. Pair with `cargo test --test release_data_guard` in `src-tauri` for the runtime baseline. See `src-tauri/src/db/README.md` §7.

- **`scripts/build-release.cmd`** (double-click) / **`pnpm build:release`** — One-click release packaging: front-end build → both data guards → `pnpm tauri build -b msi,nsis` → collect installers into `release/`. PowerShell wrapper `scripts/build-release.ps1` (UTF-8 BOM required by Windows PowerShell 5.1) supports `-Publish` (push a `v<version>` tag to trigger GitHub Actions instead of local compile), `-DryRun`, `-SkipInstall`, `-SkipGuards`, `-OutDir`. Local first build auto-downloads WiX/NSIS and takes ~10–20 min. Mirrors the `release.yml` pipeline.

- **Lint / format / type-check** — Three tools, each with one job: Prettier formats (`.prettierrc.json`), ESLint checks correctness (flat config in `eslint.config.js`), `vue-tsc` checks types. Commands: `pnpm lint`, `pnpm lint:fix`, `pnpm format`, `pnpm format:check`, `pnpm typecheck`. Rust is linted by `pnpm lint:rust` (`cargo fmt --check` + `clippy`). ESLint/Prettier dependencies are **not yet** in `package.json` — install them once with the `pnpm add -D` command in `.codebuddy/rules/lint-rules.mdc` §2 before using these commands.

- **Tests** — No test framework (Vitest/Jest/cargo-test) is configured yet. To add one: frontend use Vitest (matches Vite); backend use `cargo test` in `src-tauri`. Until then there is no single-test command; do not invent one.

## Architecture

The app follows Tauri's two-process model: a **Rust system shell** (`src-tauri/`) owns the OS window, file system, and native APIs, while a **WebView frontend** (`src/`) renders the UI in a Chromium-based WebView2. They communicate only through Tauri's IPC boundary, never by shared memory.

### Frontend layer (`src/`)
- `main.ts` mounts the root `App.vue` into `#app`. `App.vue` is currently the default greeting template and is the integration point for the pet canvas and settings UI.
- **Vue 3 `<script setup>` SFCs** are the component model (per `tsconfig`/`package.json`). The design spec mentions React for the speech bubble, but the canonical, already-scaffolded stack is **Vue 3** — new UI must use Vue, not React.
- **PIXI.js** is the planned rendering engine. The spec defines an `IPetRenderer` interface (`mount`, `playAction`, `setExpression`, `lookAt`, `getHitRegion`, `dispose`) so the code-drawn `CodeCatRenderer` and a future `CustomSpriteRenderer` are swappable. This interface is the key extension point for custom avatars (Phase 3).
- The frontend must **never** hold secrets: API keys are passed to Rust via IPC and stored encrypted; the frontend only renders.

### Backend layer (`src-tauri/`)
- `src/main.rs` is a thin binary that calls `七七_lib::run()` from `src/lib.rs`.
- `src/lib.rs` builds the Tauri app, registers plugins (`tauri_plugin_opener`), and declares `#[tauri::command]` functions in `invoke_handler!`. New native capabilities (window dragging, click-through, system probes, LLM proxy, DB access) are added here as commands.
- `Cargo.toml` pins the dependency set: `tauri` 2, `tauri-plugin-opener`, `serde`/`serde_json`, and crucially `rusqlite` (SQLCipher-vendored) + `sqlite-vec` for encrypted local storage with vector search.
- `capabilities/default.json` is the **permission gate** for IPC: it lists which core/plugin permissions the window may use (`core:default`, `opener:default` today). Any new command or plugin API the frontend calls must be permitted here, or the call fails at runtime.

### IPC boundary
Frontend invokes Rust via `import { invoke } from "@tauri-apps/api/core"`. The contract is: command name + typed args in `lib.rs`, permissions in `capabilities/`. This is the only sanctioned cross-layer channel.

### Config-as-code
The app's behavior is split across several declarative config files (see Configuration Management):
- `src-tauri/tauri.conf.json` — product name, `identifier` (`com.brave.七七`), window definitions, bundle targets, CSP/security.
- `vite.config.ts` — dev server port `1420`, strict port, HMR, and the rule to ignore `src-tauri` changes.
- `tsconfig.json` / `tsconfig.node.json` — strict TypeScript; `noUnusedLocals`/`noUnusedParameters` are on.
- `package.json` — scripts and dependency versions (single source of truth for frontend deps).

### Target architecture (from `docs/design/桌宠.md`)
The scaffold is intended to grow into:
1. **Dual-window model** — Window A: a small (≈400×400), borderless, transparent, always-on-top pet layer that only renders PIXI + speech bubbles and is hidden from the taskbar. Window B: a normal settings window (never destroyed, only `hide`n). `tauri.conf.json` currently has a single 800×600 window; the dual windows are not yet configured.
2. **Rust-owned window mechanics** — Dragging is handled by the OS (`window.start_dragging()`), not by moving PIXI sprites. Dynamic **click-through** toggles `set_ignore_cursor_events` based on the pet's hit-region (reported by frontend) vs. global mouse position (Win32 `GetCursorPos`).
3. **System probes** — Rust polls idle time (`GetLastInputInfo`), active process name (for "do not disturb while in VSCode/games"), and time windows (not absolute clocks) to drive proactive interactions.
4. **Encrypted data layer** — SQLCipher DB (key in OS credential store) with tables: `messages`, `summaries`, `facts`, `vec_memory` (sqlite-vec virtual table), `pet_states`, `affection_logs`, `app_settings`, `diaries`. Schema version tracked via `PRAGMA user_version`; migrations via `rusqlite_migration`, additive/idempotent, with `.db` backup before forward migration. `vec_memory` is a rebuildable derived table.
5. **LLM + memory pipeline** — Rust proxies OpenAI-compatible SSE streams (BYOK or built-in quota); frontend renders typewriter bubbles. After a turn, a cheap LLM extracts facts/summaries and writes vectors asynchronously (non-blocking).
6. **Affection & relationship engine** — 0–1000 points across Lv.1–4; elastic decay that stops at the current level floor; all numeric changes expressed as emotion in prompts, never raw numbers.
7. **Proactive interaction** — Micro-random engine gated by frequency tier (quiet/normal/sticky), context sensing, and time windows.

When implementing, read `docs/design/桌宠.md` alongside the code; the doc is the spec and this file is the map. The gap between the two is the backlog. Also honor `docs/design/rule.md` (Tauri-2-only APIs, Rust/Vue/PIXI code-style, SQLCipher migration rules) — it is authoritative for how code must be written.

## Configuration Management (集中配置管理)

To eliminate duplication and drift, configuration is **centralized by domain** with a single source of truth per concern. Do not hardcode values that belong in the table below into components.

| Domain | Source of truth | Scope / fields |
| :--- | :--- | :--- |
| App identity & bundle | `src-tauri/tauri.conf.json` (`productName`, `identifier`, `version`, `bundle`) | Product name `七七`; identifier (currently placeholder `com.brave.七七`); icon set; build targets. |
| Window definitions | `src-tauri/tauri.conf.json` `app.windows` | Window A (`main`: 400×400 transparent/always-on-top/skipTaskbar) + Window B (`settings`: 1000×660 self-drawn title bar, `decorations:false`). Labels are the front/back contract (`src/main.ts` dispatch + `window/manager.rs` constants). Window B is `visible:true` during dev (pre-tray); C2.6 flips it to `false`. Close→`hide` interception lives in `lib.rs` `on_window_event`. |
| Security / CSP | `src-tauri/tauri.conf.json` `app.security` | CSP policy; currently `null` (permissive for dev). Tighten before release. |
| IPC permissions | `src-tauri/capabilities/default.json` | B2 权限矩阵（v1.0）：显式 `windows:["main","settings"]`；core 权限 = default + window(start-dragging/hide/show/set-focus/set-ignore-cursor-events) + event(listen/emit/unlisten)；插件仅 `opener:default`。**自定义命令免授权**（勘误#1）；global-shortcut/autostart 仅 Rust 侧无 JS 条目；store/dialog/fs 不预授权。矩阵全表见接口契约 §4.1，最小授权红线：禁通配禁预授权，新增前端 API 时在子计划中逐项申请 |
| Dev server | `vite.config.ts` | Port `1420` (strict), HMR, `src-tauri` watch-ignore. |
| TypeScript | `tsconfig*.json` | Strict mode; unused-local/param checks. |
| Frontend deps & scripts | `package.json` | Single authority for versions and `dev`/`build`/`tauri`/`preview` scripts. |
| User runtime settings | SQLCipher `app_settings` table (JSON `value`) | `llm_base_url`, `llm_api_key`, `llm_model`, `llm_model_custom`, `dnd_enabled`, `dnd_start_time`, `dnd_end_time`, `is_custom_quota_used`. Encrypted; frontend never stores plaintext. |
| LLM provider presets | Centralized constant (not in components) | OpenAI / DeepSeek / Kimi / 通义千问 base URLs + default models. Define once, reuse in settings UI. |
| Pet state | SQLCipher `pet_states` | `pet_name`, `relationship_level`, `affection_points`, `active_renderer`, `renderer_config`, `last_interaction`, `last_decay_at`. |
| UI design tokens | `docs/design/pet-design/index.html` `:root` CSS variables | Jelly-Toy palette (`--matcha`, `--peach`, `--butter`, radii, shadows, fonts). Extract to a shared tokens source when building the real UI to avoid color drift. |
| DB schema version | `PRAGMA user_version` | Integer schema version; migrations in Rust. |

**CodeBuddy config-file fields (this document):** This `CODEBUDDY.md` is the agent-facing config. Its required fields are: *Project Overview*, *Commands*, *Architecture*, *Configuration Management* (above), and *Documentation Conventions* (below). Its scope is to keep CodeBuddy aligned with build commands, the cross-file architecture, and the canonical config locations — not to replace the design spec or runtime config.

**Sync rule:** When you change `tauri.conf.json`, `package.json`, `vite.config.ts`, `capabilities/`, or the `app_settings`/`pet_states` schema, update the matching row above and any affected command/architecture text. Config values must have exactly one home; if you find the same value in two places, the table above wins.

## Documentation Conventions (文档组织与命名规范)

Documentation is centralized under the project so it travels with the code:

- **`README.md`** (repo root) — entry point: what the app is and how to install/run/build. Keep it short; link to `CODEBUDDY.md` for agent/dev detail.
- **`docs/design/`** — all design/requirement docs, now moved in-repo:
  - `桌宠.md` — v2.0 technical spec (architecture, modules, DB schema, affection/memory systems). The product specification.
  - `rule.md` — development rules: Tauri-2-only APIs, Rust/Vue/PIXI code style, SQLCipher migration rules. Authoritative for how code must be written.
  - `pet-design/index.html` — settings-window (Window B) mockup with the Jelly-Toy design tokens; its `assets/` sit beside it.
- **Naming** — English filenames use `kebab-case`; Chinese design docs keep their descriptive Chinese names with a version suffix in the title (e.g. "技术开发文档 v2.0"). Code identifiers follow the existing stacks (Rust `snake_case`, TypeScript/Vue `camelCase`/`PascalCase`).
- **Single source of truth** — design intent lives in `docs/design/桌宠.md` + `rule.md`; runtime config lives in the config table above; code is the implementation. Do not duplicate spec text into `CODEBUDDY.md` — summarize and link.
- **Keeping in sync** — `CODEBUDDY.md` must reflect the config table and commands exactly. Before merging config changes, verify this file still matches `tauri.conf.json` / `package.json` / `capabilities/`.
