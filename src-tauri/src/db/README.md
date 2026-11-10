# 数据层（`src-tauri/src/db`）

本文件是数据库表相关文件的**唯一索引与操作约定**。DDL 真源在 `migrations/`，
访问真源在 `repositories/`；本文件只描述"东西在哪"和"改表时该动哪些文件"，
不重复抄写字段定义，避免多处定义漂移。

技术栈：SQLite + SQLCipher（全库加密）+ sqlite-vec，迁移由 `rusqlite_migration`
按 `PRAGMA user_version` 管理。规范依据见 `.codebuddy/rules/design-baseline.mdc` §E。

## 1. 文件职责地图

| 路径 | 职责 | 何时修改 |
| :--- | :--- | :--- |
| `README.md` | 本文件：表清单、职责地图、改表 SOP、命名与红线 | 任何表结构变更 |
| `mod.rs` | 模块声明与 `Db` / `init` 重导出；不含业务 SQL | 新增模块时 |
| `connection.rs` | SQLCipher 密钥、sqlite-vec 扩展注册、连接打开、版本三分支与迁移前备份 | 连接策略变化时 |
| `migrations.rs` | 有序迁移链（13 条）、`TARGET_VERSION`、迁移链测试 | **每次表结构变更** |
| `migrations/M*.sql` | 各版本 DDL / 数据迁移 / 种子，静态文件 | 新增迁移时 |
| `models/rows.rs` | SQL 行结构体（内部使用，不序列化） | 新增/改表且需行映射时 |
| `models/values.rs` | `app_settings.value` 列的 JSON 存储结构（含旧库兼容读取） | 改配置结构时 |
| `models/dto.rs` | 命令层入参/出参 DTO（serde camelCase） | 新增/改 IPC 契约时 |
| `models/mod.rs` | 三者 `pub use` 重导出，固定对外路径 `crate::db::models::X` | 增删模型文件时 |
| `repositories/*.rs` | 按表分文件，只写参数化 SQL，不做业务决策、不 emit 事件 | 新增/改表时 |
| `repositories/mod.rs` | 仓储模块声明 + 两张模型表共用的 JSON 解析/序列化出口 | 新增仓储时 |
| `../../tests/release_data_guard.rs` | 发布数据运行时守卫：干净库断言仅含系统基础数据 | 基线变化时 |
| `../../../scripts/verify-release-data.mjs` | 发布数据构建期守卫：扫描构建输入中的数据文件 | 发布链路变化时 |

## 2. 表清单（17 张）

| 表 | 用途 | 建表迁移 | 仓储 | 行结构体 |
| :--- | :--- | :--- | :--- | :--- |
| `messages` | 对话记录（按 session_id） | M1 | `messages.rs` | — |
| `summaries` | 会话摘要 | M1 | `summaries.rs` | — |
| `facts` | 结构化事实（蒸馏结果） | M1 | `facts.rs` | `FactRow` |
| `vec_memory` | 向量索引（vec0 虚拟表，**派生可重建**） | M2 | `vec_memory.rs` | — |
| `pet_states` | 桌宠状态，单行 `id=1` | M1 | `pet_states.rs` | `PetStatesRow` |
| `affection_logs` | 好感度流水 | M1 | `affection_logs.rs` | `AffectionLogRow` |
| `app_settings` | 全局配置 JSON（`type` 1/2/3 分行） | M1 | `app_settings.rs` | `AppSettingsRow` |
| `diaries` | 每日日记 | M1 | `diaries.rs` | `DiaryRow` |
| `custom_model_settings` | 用户自定义大模型（BYOK） | M5 | `custom_model_settings.rs` | `CustomModelRow`（就近） |
| `default_model_settings` | 内置免费额度模型（只读） | M6 | `default_model_settings.rs` | `DefaultModelRow`（就近） |
| `quota_usage` | 内置额度用量明细（含平台名） | M8 + M10 加列 | `quota_usage.rs` | `QuotaUsageDetailRow` |
| `quota_usage_monthly` | 额度月度全局汇总缓存 | M8 | `quota_usage.rs` | `QuotaUsageMonthlyRow` |
| `quota_usage_platform_monthly` | 按平台拆分的月度用量（`period_key+platform` 主键） | M10 | `quota_usage.rs` | `QuotaPlatformMonthlyRow` |
| `search_settings` | 联网搜索配置，单行 `id=1` | M9 | `search_settings.rs` | — |
| `translate_settings` | 划词翻译配置，单行 `id=1`（M11 建表、M12 播内置凭据） | M11 + M12 | `translate_settings.rs` | — |
| `custom_translate_providers` | 用户自填的多个翻译服务商（自填优先） | M12 | `custom_translate_providers.rs` | `CustomTranslateProviderRow`（就近） |
| `builtin_translate_providers` | 内置翻译服务商（百度 + 讯飞）凭据、启停与各自月度上限 | M13 | `builtin_translate_providers.rs` | `BuiltinProviderRow`（就近） |

说明：

- `quota_usage.rs` 一个仓储管三张表（明细 + 全局汇总 + 平台汇总，同事务 upsert），故仓储数小于表数。
- `translate_settings` 的内置百度凭据是 **M12 已登记的生产凭据种子**（与 M6 同口径，只存在于 Rust 侧 SQL）；
  构建期环境变量可覆盖它以便轮换，未注入时 M12 种子即可开箱可用。
  `custom_api_key` 等用户自填列首发为空；干净库首次启动后该表 1 行。
- `builtin_translate_providers` 是 M13 起**内置翻译的唯一凭据真源**（百度 / 讯飞各一行），
  `name` 同时是 `quota_usage_platform_monthly.platform` 键；`monthly_granted` / `is_can_use`
  由 M13 种子在**底层固定**，仅作安全兜底——设置页不暴露上限配置与启停开关，也不注册写命令，
  防单家过量靠按当月用量自动均衡。凭据同为已登记的生产凭据，只存在于 Rust 侧迁移 SQL；干净库首发 2 行。
  M13 文件头注释里的「设置页可改」是初始设计，按红线不修改已执行的历史 SQL，以本条为准。
  `translate_settings.builtin_*` 保留为 M12 历史列（兼容旧库与 `has_builtin_credential` 视图），
  不再参与内置选路；构建期注入 `QIQI_BAIDU_APPID` / `QIQI_BAIDU_SECRET` 时，
  `connection::apply_migrations` 会同步覆盖 M13 的百度行，保持 env 轮换能力。
- `quota_usage.platform` 列由 M10 追加（存量按 `model` 反查内置模型回填，查不到 `unknown`）；
  M8 历史 SQL 不改动。
- `CustomModelRow` / `DefaultModelRow` 含明文 `api_key`，**刻意就近定义在仓储内**，不放进
  `models/rows.rs`，避免经 `pub use` 重导出扩散到全 crate。
- 无独立 Row 的表由仓储直接投影为 DTO 或使用局部类型。

## 3. 迁移链（M1–M13）

| 版本 | 文件 / 来源 | 类型 | 内容 |
| :--- | :--- | :--- | :--- |
| M1 | `M1__init_core.sql` | DDL + 种子 | 7 张核心表、4 个索引、幂等种子 |
| M2 | **`migrations.rs::chain()` 代码渲染** | DDL | `vec_memory` vec0 虚拟表 |
| M3 | `M3__llm_tiers.sql` | 数据迁移 | `app_settings` 扁平单档 → 三档 tiered |
| M4 | `M4__dnd_tiers.sql` | 数据迁移 | 旧单段勿扰 → 多段 + frequency |
| M5 | `M5__custom_models.sql` | DDL | 自定义模型表 |
| M6 | `M6__default_models.sql` | DDL + 种子 | 内置模型表、额度口径切 BYOK |
| M7 | `M7__clear_llm_tiers.sql` | 数据迁移 | 清空三档（UI 合并为平台列表） |
| M8 | `M8__quota_usage.sql` | DDL | 额度明细 + 月度汇总 |
| M9 | `M9__search_settings.sql` | DDL + 种子 | 联网搜索配置 |
| M10 | `M10__quota_platform.sql` | DDL + 数据回填 | 额度明细加 platform 列、平台月表、存量回填、`granted` 对齐 100,000 |
| M11 | `M11__translate_settings.sql` | DDL（仅建表） | 划词翻译配置单行表 |
| M12 | `M12__translate_providers.sql` | 种子 + DDL | 内置百度翻译凭据幂等种子、用户自填多服务商表 |
| M13 | `M13__builtin_translate_providers.sql` | DDL + 种子 | 内置翻译服务商表、百度 / 讯飞凭据与各自月度上限 |

**M2 没有 SQL 文件是有意设计，不是遗漏**：vec0 的 `float[dim]` 必须与
`config::EMBEDDING_DIM` 单点一致，而 `M::up` 只接受 `&'static str`，故在
`chain()` 内用 `format!` 渲染后 `Box::leak`。请勿补一个静态 M2 文件。

`TARGET_VERSION = 13`（= 链上迁移条数），由 `connection::apply_migrations` 用于
版本三分支：相等跳过 / 大于失败关闭 / 小于（且有数据）先备份 `pet.db.bak` 再迁移。

## 4. 新增或修改数据表的操作路径

### 4.1 新增表

1. 在 `migrations/` 新建 `M{N}__{snake_case}.sql`，`N` 取当前 `TARGET_VERSION + 1`。
   命名统一为 `M{版本}__{描述}.sql`（双下划线、snake_case、版本号连续）。
2. SQL 必须幂等：`CREATE TABLE IF NOT EXISTS` / `CREATE INDEX IF NOT EXISTS`；
   种子用 `INSERT ... SELECT ... WHERE NOT EXISTS(...)` 守卫。
3. 在 `migrations.rs::chain()` **末尾追加**一条 `M::up(include_str!(...))`；
   同时把 `TARGET_VERSION` 加 1。（链顺序即版本号，禁止插队、禁止改历史 SQL。）
4. 在 `migrations.rs` 的 `#[cfg(test)]` 补一条覆盖新迁移的测试（存量库升级 + 幂等）。
5. 在 `repositories/` 新建同表名文件，并在 `repositories/mod.rs` 声明模块；
   SQL 全部走 `params![]`，错误一律 `AppError::Db`。
6. 需要行映射时在 `models/rows.rs` 加 `XxxRow`；需要跨 IPC 时在 `models/dto.rs`
   加 DTO 并**同步 `src/api/types.ts`**（camelCase 逐字段对齐）。
7. 更新本文件 §2 表清单与 §3 迁移链；若新表带种子数据，**同步更新 §7 发布数据基线**
   与 `tests/release_data_guard.rs` 的期望值，否则发布会被引擎阻断。
8. 跑校验：`pnpm lint:rust`（fmt + clippy）、`cd src-tauri && cargo test`、
   前端 `pnpm typecheck` / `pnpm test`、`pnpm verify:release-data`。

### 4.2 修改已有表

- **只追加新迁移**，绝不修改已执行的 `M*.sql`（存量库不会重跑，改了等于没改）。
- 优先加量：`ALTER TABLE ... ADD COLUMN` / `CREATE TABLE IF NOT EXISTS`。
- 必须删列或改类型时：新建临时表 → `INSERT INTO SELECT` → DROP 旧表 → RENAME，
  全程独立事务包裹；`vec_memory` 作为派生表直接 `DROP` 后从 `facts`/`messages` 重建。
- 改 `app_settings.value` 结构时：在 `models/values.rs` 加新结构，并保留旧格式的
  兼容读取（`Legacy*`）；数据转换写成新的数据迁移（参照 M3 / M4 / M7）。

### 4.3 不要做的事

- 不改历史迁移 SQL、不手工覆写 `PRAGMA user_version`。
- 不把业务 SQL 写进 `connection.rs` / `migrations.rs` / `mod.rs`。
- 不在仓储里 emit 事件、不做业务决策、不拼字符串 SQL。
- 不把含 `api_key` 的行结构体或明文 Key 放进 DTO / 日志 / 错误信息。
- 不把 PIXI / Tauri 类型带进本目录；本层只依赖 `rusqlite` / `serde` / `AppError`。

## 5. 命名约定

| 对象 | 约定 | 示例 |
| :--- | :--- | :--- |
| 迁移文件 | `M{版本}__{描述}.sql` | `M8__quota_usage.sql` |
| 仓储文件 | 与表名一致，snake_case | `quota_usage.rs` |
| 行结构体 | `{表}Row` | `PetStatesRow` |
| 存储结构 | `{语义}Value` / `Legacy{语义}Value` | `TieredValue` / `LegacyFlatValue` |
| 出参 DTO | `{语义}Dto` | `PetStateDto` |
| 入参 DTO | `Save{语义}Payload` / `Update{语义}Payload` | `SaveTierPayload` |
| 模块/函数 | snake_case；Tauri 命令动词开头 | `pet_states::set_affection` |

## 6. 已知遗留（本次整理未改动，登记备查）

- `M6__default_models.sql` 内含内置模型平台的明文 API Key 种子。它属于已执行的
  历史迁移，按红线不可修改；如需轮换，应新增迁移 `UPDATE` 并同步评估种子口径。
- `M3` / `M4` / `M7` 是历史数据迁移，链上必经执行，不可删除。
- `models/values.rs` 的 `Legacy*` 结构仍被读取端引用，用于兜底极旧库，不可删。
- `M6` 的两个内置模型平台明文 Key 在发布守卫中登记为**允许的系统基础数据**（见 §7.5）。

## 7. 发布数据基线与防回归

### 7.1 数据是怎么进安装包的

安装包**不含任何 `.db` 文件**：`tauri.conf.json` 未配置 `bundle.resources`，
`build.rs` 无拷贝逻辑，`lib.rs` 在 `app_data_dir()/pet.db` 首次启动现场建库。
种子以 `include_str!` 编译进二进制，由迁移链 M1–M13 写入。

因此治理对象是**迁移 SQL 与构建输入**，不是磁盘上的库文件；"自动排除测试数据"
落实为「构建期扫描 + 运行时基线断言 + CI 阻断」，而不是打包时清理 `.db`。

> 开发机反复安装时看到的旧聊天 / 记忆 / 好感度，是
> `%APPDATA%\com.brave.qiqi\pet.db` 未被卸载程序清除，**不是安装包自带**。
> 验收必须按 §7.4 用全新用户目录。

### 7.2 生产基线（干净库首次启动后）

| 表 | 允许行数 | 判定 |
| :--- | ---: | :--- |
| `pet_states` | 1 | 系统基础：默认「喵喵」/ `code_cat` / Lv1 / 0 分 |
| `app_settings` | 2 | 系统基础：type=1 空三档；type=2 勿扰 + `frequency=normal` |
| `default_model_settings` | 2 | 系统基础：智谱 / 硅基流动内置额度模型 |
| `search_settings` | 1 | 系统基础：`tavily` / 未启用 / 空 Key |
| `translate_settings` | 1 | 系统基础：`baidu` / `auto` / 默认开启 / 空自定义 Key（内置凭据由 M12 种子写入） |
| `builtin_translate_providers` | 2 | 系统基础：百度 / 讯飞内置凭据 + 各自月度上限（M13 幂等种子） |
| `messages` | 0 | 运行时对话 |
| `summaries` | 0 | 运行时摘要 |
| `facts` | 0 | 运行时蒸馏事实 |
| `vec_memory` | 0 | 运行时向量索引（派生可重建） |
| `affection_logs` | 0 | 运行时好感度流水 |
| `diaries` | 0 | 运行时日记 |
| `custom_model_settings` | 0 | 用户自配，首发不得预置 |
| `custom_translate_providers` | 0 | 用户自配翻译服务商，首发不得预置 |
| `quota_usage` | 0 | 运行时用量明细 |
| `quota_usage_monthly` | 0 | 运行时月度全局汇总 |
| `quota_usage_platform_monthly` | 0 | 运行时按平台月度汇总（M10 新增，干净库无数据） |

合计 17 张表、**9 行系统基础数据**（`translate_settings` 的内置凭据由 M12 幂等种子写入，
`builtin_translate_providers` 两家凭据由 M13 幂等种子写入，与 M6 同属已登记的生产凭据；
`custom_translate_providers` 首发 0 行）。任何标注为 0 行的运行时表出现非 0 行，即视为
测试 / 临时数据混入，发布必须阻断。

**禁止出现在发布构建中的标记**（守卫会扫描迁移 SQL 与 `dist/`）：
`sk-old`、`old/v1`、`TEST_`、`test-fixture`、`mock-data`、以及任何
`*.db` / `*.sqlite*` / `*.bak` / `*.corrupt-*` 文件。

### 7.3 四层防回归

| 层 | 落点 | 触发 | 作用 |
| :--- | :--- | :--- | :--- |
| 1 定义 | 本文件 §7.2 | 人工维护 | 基线的单一真源，改表时同步 |
| 2 静态守卫 | `scripts/verify-release-data.mjs` | `pnpm verify:release-data` | 校验 `bundle.resources` 为空、扫描源码/`dist` 无数据文件；`dist` 命中自动删除并报告，源码命中直接失败；扫描迁移 SQL 无测试标记、INSERT 白名单 |
| 3 运行时守卫 | `src-tauri/tests/release_data_guard.rs` | `cargo test --test release_data_guard` | 干净空库跑完 M1–M13，断言表集合、逐表行数、关键默认值与无测试串 |
| 4 CI 阻断 | `.github/workflows/release.yml` | 打 `v*` tag | 在 `tauri-action` **之前**依次跑 2、3，任一失败即中止、不产出 Release |

新增迁移若改变行数或表集合，必须同步改 §7.2 与 `release_data_guard.rs`
的期望表；否则守卫会失败——这是刻意的"改表必经"闭环（呼应 §4.1 第 7 步）。

### 7.4 全新用户目录验收（排除开发机残留）

MSI 重装不会删除 `app_data_dir`，开发机上验证发布包时必须先清数据：

1. 完全退出应用（托盘 → 退出），确认进程结束。
2. 备份（可选）后删除数据目录：
   `Remove-Item -Recurse -Force "$env:APPDATA\com.brave.qiqi"`
3. 安装新包并**首次启动**（触发 M1–M12 现场建库）。
4. 打开设置 → 记忆 / 好感度：记忆为空、事实 0 条、流水 0 条、好感度 0 分 / Lv1。
5. 桌宠名应为默认「喵喵」、形象 `code_cat`；大模型与联网搜索均未配置 Key。

若不清目录，旧库会走前向迁移分支（先备份 `pet.db.bak`）而**保留历史数据**，
这是预期行为，不代表安装包带数据。

### 7.5 已登记例外

`M6__default_models.sql` 的两个明文 API Key 属「内置免费额度模型」的
生产凭据：移除会让内置额度功能失效，本期按用户决策**保留**，仅在此登记。
因此：

- 守卫把 `default_model_settings` 的 2 行计入允许基线；
- 但静态守卫仍会阻断这两个 Key 出现在 `dist/`（前端产物绝不含凭据）；
- 后续如改为服务端下发，须同步更新 §7.2 基线与守卫期望值。
