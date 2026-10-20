开发语言使用规则

+ <font style="color:rgb(15, 17, 21);">“你是一个精通 Tauri 2、Rust 和 PIXI.js 的资深专家。请始终使用 Tauri 2 的 API（如 </font>`<font style="color:rgb(15, 17, 21);background-color:rgb(235, 238, 242);">@tauri-apps/api/core</font>`<font style="color:rgb(15, 17, 21);">），不要使用 Tauri 1 的旧 API。”</font>
+ <font style="color:rgb(15, 17, 21);">“前端与 Rust 的分工原则：所有涉及文件系统、系统底层检测、窗口透明/穿透的操作，必须使用 Rust 实现，并通过 Tauri IPC 供前端调用。前端只负责 UI 和 PIXI 渲染。”</font>
+ <font style="color:rgb(15, 17, 21);">“PIXI.js 是命令式引擎。在 React/Vue 中初始化 PIXI 时，必须使用 </font>`<font style="color:rgb(15, 17, 21);background-color:rgb(235, 238, 242);">ref</font>`<font style="color:rgb(15, 17, 21);"> 或独立类封装，绝对不要把 PIXI 实例放入响应式状态（useState/reactive）中，防止 Proxy 拦截导致性能灾难。”</font>
+ <font style="color:rgb(15, 17, 21);">“数据库使用 SQLite + SQLCipher + sqlite-vec。Rust 侧负责执行 SQL 和加密，前端仅通过 command 请求数据。”</font>



数据库结构与迁移规则

1. <font style="color:rgb(15, 17, 21);">版本追踪：数据库 schema 版本严格使用 </font>`<font style="color:rgb(15, 17, 21);">PRAGMA user_version</font>`<font style="color:rgb(15, 17, 21);"> 追踪，禁止自建 version 数据表。</font>
2. <font style="color:rgb(15, 17, 21);">强制迁移：任何对表结构的修改（新增表、新增字段、修改索引），必须通过 </font>`<font style="color:rgb(15, 17, 21);">rusqlite_migration</font>`<font style="color:rgb(15, 17, 21);"> 库新增一条迁移脚本，严禁直接修改已有的历史建表 SQL。</font>
3. <font style="color:rgb(15, 17, 21);">加量优先：优先使用 </font>`<font style="color:rgb(15, 17, 21);">CREATE TABLE IF NOT EXISTS</font>`<font style="color:rgb(15, 17, 21);">、</font>`<font style="color:rgb(15, 17, 21);">ALTER TABLE ADD COLUMN</font>`<font style="color:rgb(15, 17, 21);"> 等幂等操作。严禁直接使用 </font>`<font style="color:rgb(15, 17, 21);">DROP COLUMN</font>`<font style="color:rgb(15, 17, 21);"> 或重命名表。</font>
4. <font style="color:rgb(15, 17, 21);">安全重塑：若必须删除列或修改列类型，必须采用“新建临时表 -> INSERT INTO SELECT 拷贝数据 -> DROP 旧表 -> RENAME 临时表”的流程，并且整个流程必须包裹在独立的事务中。</font>
5. <font style="color:rgb(15, 17, 21);">派生数据重建：</font>`<font style="color:rgb(15, 17, 21);">vec_memory</font>`<font style="color:rgb(15, 17, 21);"> 等向量虚拟表属于“可重建的派生数据”。遇到 </font>`<font style="color:rgb(15, 17, 21);">sqlite-vec</font>`<font style="color:rgb(15, 17, 21);"> 扩展升级或维度变更导致不兼容时，直接在迁移脚本中 DROP 并重建该表，从 </font>`<font style="color:rgb(15, 17, 21);">facts</font>`<font style="color:rgb(15, 17, 21);">/</font>`<font style="color:rgb(15, 17, 21);">messages</font>`<font style="color:rgb(15, 17, 21);"> 重新生成向量，严禁试图去修改虚拟表结构。</font>
6. <font style="color:rgb(15, 17, 21);">迁移前置动作：执行数据库迁移前，必须先对当前 </font>`<font style="color:rgb(15, 17, 21);">.db</font>`<font style="color:rgb(15, 17, 21);"> 文件进行复制备份（如 </font>`<font style="color:rgb(15, 17, 21);">pet.db.bak</font>`<font style="color:rgb(15, 17, 21);">）。</font>

<font style="color:rgb(15, 17, 21);"></font>

<font style="color:rgb(15, 17, 21);">通用开发与架构规则（General Rules）</font>

+ <font style="color:rgb(15, 17, 21);">严格的边界隔离：前端绝不直接调用系统底层 API（不引 Node.js 原生模块）。所有涉及文件系统、全局系统检测、窗口拖拽/透明穿透的操作，必须用 Rust 实现，通过 #[tauri::command] 暴露给前端。</font>
+ <font style="color:rgb(15, 17, 21);">安全第一：API Key 绝不写入前端的 .env 或 localStorage。</font>
+ <font style="color:rgb(15, 17, 21);">所有用户数据、记忆、配置必须写入 SQLCipher 加密的本地数据库。</font>
+ <font style="color:rgb(15, 17, 21);">AI 幻觉防范：强制锁定 API 版本（如 Tauri 2 的 @tauri-apps/api/core），禁止使用旧版本语法。AI 生成的第三方库调用代码，必须先查证官方文档。</font>



代码风格规则

4.1 Rust 侧代码规范

+ 错误处理：核心业务逻辑（如数据库操作、窗口调用）严禁滥用 .unwrap() 或 .expect()。必须定义统一的 AppError 枚举，使用 ? 操作符向上传播。
+ 异步编程：涉及 I/O 操作（如请求大模型、读写文件）必须使用 tokio 异步执行，避免阻塞主线程。
+ 命名规范：模块名用 snake_case，结构体/枚举用 PascalCase，函数/变量用 snake_case。
+ 数据库请求：必须使用参数化查询（如 conn.execute("INSERT INTO ... VALUES (?1)", params![val])），绝对禁止通过字符串拼接 SQL 语句。



4.2 TypeScript / React / Vue 侧规范

+ 类型严谨：开启 strict 模式，严禁使用 any。PIXI 实例、IPC 返回数据必须有明确的 Interface 定义。
+ 响应式隔离（防卡顿）：严禁将 PIXI 的 Application、Sprite、Container 等实例放入 React 的 useState 或 Vue 的 reactive 中。必须使用 useRef 或独立类实例持有。
+ 清理副作用：在 React 的 useEffect 或 Vue 的 onUnmounted 中，必须彻底销毁 PIXI 实例（调用 .destroy(true)）并移除事件监听，防止内存泄漏。



4.3 PIXI.js 渲染规范

+ 统一 Ticker：整个应用只能有一个 PIXI Ticker 循环，所有动画逻辑（呼吸、果冻形变、视线跟随）都在此循环内统一计算。
+ 防 GC 卡顿：在 Ticker 的每帧更新中，严禁创建新对象（如每帧 new Point()）。所有临时计算变量应在循环外部声明并复用。

<font style="color:rgb(15, 17, 21);"></font>

工作流与开发规范

+ 先设计，后生成：让 AI 写代码前，先让它给出架构设计或伪代码，确认无误后再让它写完整实现。
+ 隔离测试：AI 生成的 Rust 数据库代码或 Tauri IPC 代码，优先在独立的测试文件中跑通（如写一个 #[test] 验证），再集成到主代码。
+ 冷眼审查：AI 生成完毕后，必须做三项检查：
    - 有没有漏掉 await 或错误捕获？
    - 有没有把 PIXI 实例塞进响应式状态？
    - 有没有在 Rust 里写死系统绝对路径？


