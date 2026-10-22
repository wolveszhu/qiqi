//! 数据层集成测试（B4.7 / 附录 A §A.7 七项用例）。
//!
//! 基础设施：`tempfile` 临时目录 + 固定测试密钥（`init_with_key` 旁路 keyring）——
//! 绝不触碰用户真实数据目录；每用例独立临时库，互不影响。

// F1.4：集成测试 fixture 用 unwrap/expect/index 属合理用法，统一豁免四项 clippy lint。
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use rusqlite::Connection;

use qiqi_lib::db::connection::{Db, init_with_key, register_vec_extension};

/// 固定测试密钥（64 hex 字符，同生产 32 字节 hex 形态）。
const TEST_KEY: &str = "0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF";

/// 新建临时库并完成全链迁移。
fn fresh_db() -> (tempfile::TempDir, Connection) {
    register_vec_extension().expect("vec 注册");
    let dir = tempfile::tempdir().expect("临时目录");
    let db_path = dir.path().join("pet.db");
    let Db { conn } = init_with_key(&db_path, TEST_KEY).expect("初始化");
    (dir, conn.into_inner().expect("取连接"))
}

// ---------- 用例 1：空库全链迁移 ----------

#[test]
fn case1_empty_db_migrates_full_chain() {
    let (_dir, conn) = fresh_db();

    // 8 张表存在（7 普通表 + vec_memory 虚拟表）
    for table in [
        "messages",
        "summaries",
        "facts",
        "pet_states",
        "affection_logs",
        "app_settings",
        "diaries",
        "vec_memory",
    ] {
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type IN ('table','view') AND name = ?1",
                [table],
                |row| row.get(0),
            )
            .expect("查询");
        assert!(exists, "表缺失: {table}");
    }

    // 4 个索引存在
    for index in [
        "idx_messages_session_time",
        "idx_facts_category",
        "idx_aff_logs_time",
        "idx_app_settings_type",
    ] {
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type = 'index' AND name = ?1",
                [index],
                |row| row.get(0),
            )
            .expect("查询");
        assert!(exists, "索引缺失: {index}");
    }

    // user_version == 3（M1+M2+M3；M3 为勘误 #7 数据迁移）
    let version: usize = conn.query_row("PRAGMA user_version", [], |row| row.get(0)).expect("版本");
    assert_eq!(version, 3);

    // 种子：pet_states 仅 id=1 一行
    let pet_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM pet_states", [], |row| row.get(0)).expect("查询");
    assert_eq!(pet_count, 1);

    // 种子：app_settings type=1/2 各一行且 value 可 JSON 解析
    for ty in [1_i64, 2_i64] {
        let value: String = conn
            .query_row("SELECT value FROM app_settings WHERE type = ?1", [ty], |row| row.get(0))
            .expect("settings 行");
        serde_json::from_str::<serde_json::Value>(&value).expect("value 应可 JSON 解析");
    }
    // 勘误 #7：type=1 种子应为三档 tiered 结构（新库直接匹配 B5 DTO）
    let tiered: String = conn
        .query_row("SELECT value FROM app_settings WHERE type = 1", [], |row| row.get(0))
        .expect("settings 行");
    let parsed: serde_json::Value = serde_json::from_str(&tiered).expect("JSON 解析");
    assert!(parsed.get("main").is_some(), "种子应含 main 档: {tiered}");
    assert!(parsed.get("memory").is_some(), "种子应含 memory 档: {tiered}");
    assert!(parsed.get("fallback").is_some(), "种子应含 fallback 档: {tiered}");
    let settings_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM app_settings", [], |row| row.get(0)).expect("查询");
    assert_eq!(settings_count, 2);
}

// ---------- 用例 2：重复执行幂等 ----------

#[test]
fn case2_rerun_migrations_is_idempotent() {
    register_vec_extension().expect("vec 注册");
    let dir = tempfile::tempdir().expect("临时目录");
    let db_path = dir.path().join("pet.db");

    // 首次初始化
    let first = init_with_key(&db_path, TEST_KEY).expect("首次初始化");
    drop(first);

    // 二次初始化（版本相等路径）：应无备份文件、种子不重复
    let second = init_with_key(&db_path, TEST_KEY).expect("二次初始化");
    let conn = second.conn.lock().expect("锁");
    let pet_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM pet_states", [], |row| row.get(0)).expect("查询");
    assert_eq!(pet_count, 1, "种子重复插入");
    let settings_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM app_settings", [], |row| row.get(0)).expect("查询");
    assert_eq!(settings_count, 2, "设置种子重复");
    assert!(!db_path.with_extension("db.bak").exists(), "相等路径不应产生备份");
}

// ---------- 用例 3：版本三分支 ----------

#[test]
fn case3a_db_version_above_target_fails_closed() {
    register_vec_extension().expect("vec 注册");
    let dir = tempfile::tempdir().expect("临时目录");
    let db_path = dir.path().join("pet.db");
    let db = init_with_key(&db_path, TEST_KEY).expect("初始化");
    // 手工把 user_version 抬到高于目标（模拟"旧应用写的新库"）——注意：
    // 应用代码禁止覆写 user_version，这里仅测试构造场景。
    db.conn.lock().expect("锁").execute_batch("PRAGMA user_version = 99;").expect("抬版本");
    drop(db);

    let err = init_with_key(&db_path, TEST_KEY).expect_err("应失败关闭");
    assert!(matches!(err, qiqi_lib::error::AppError::Config(_)), "高于版本应报 Config: {err:?}");
}

#[test]
fn case3b_db_version_below_target_backs_up_then_migrates() {
    register_vec_extension().expect("vec 注册");
    let dir = tempfile::tempdir().expect("临时目录");
    let db_path = dir.path().join("pet.db");

    // 先只迁移到版本 1（to_version 而非 to_latest）
    {
        let mut conn = Connection::open(&db_path).expect("打开");
        conn.execute_batch(&format!("PRAGMA key = \"x'{TEST_KEY}'\";")).expect("密钥");
        qiqi_lib::db::migrations::chain().to_version(&mut conn, 1).expect("迁移到 v1");
        // 更新种子行数据，验证迁移后保留（种子已插入 id=1，不可再 INSERT）
        conn.execute("UPDATE pet_states SET pet_name = '备份前' WHERE id = 1", []).expect("写入");
    }

    // 正式初始化：应备份 + 迁移到 v2 + 数据保留
    let db = init_with_key(&db_path, TEST_KEY).expect("初始化");
    let conn = db.conn.lock().expect("锁");
    assert!(db_path.with_extension("db.bak").exists(), "应产生备份");
    let version: usize = conn.query_row("PRAGMA user_version", [], |row| row.get(0)).expect("版本");
    assert_eq!(version, 3);
    let name: String = conn
        .query_row("SELECT pet_name FROM pet_states WHERE id = 1", [], |row| row.get(0))
        .expect("数据");
    assert_eq!(name, "备份前", "迁移不应丢数据");
    let vec_exists: bool = conn
        .query_row("SELECT COUNT(*) > 0 FROM sqlite_master WHERE name = 'vec_memory'", [], |row| {
            row.get(0)
        })
        .expect("查询");
    assert!(vec_exists, "M2 应已迁移");
}

// ---------- 用例 4：SQLCipher 加密 ----------

#[test]
fn case4_db_is_encrypted() {
    let (dir, _conn) = fresh_db();
    let db_path = dir.path().join("pet.db");

    // 文件头非明文 SQLite（SQLite 明文库头部为 "SQLite format 3\0"）
    let head = std::fs::read(&db_path).expect("读取");
    assert!(!head.starts_with(b"SQLite format 3\0"), "库文件不应为明文 SQLite");

    // 无密钥打开：PRAGMA key 缺失时任何业务查询应失败（密文校验错误）
    let conn = Connection::open(&db_path).expect("无密钥打开连接本身可成功");
    let result = conn.query_row("SELECT COUNT(*) FROM pet_states", [], |row| row.get::<_, i64>(0));
    assert!(result.is_err(), "无密钥查询应失败（库已加密）");
}

// ---------- 用例 5：vec_memory 往返 ----------

#[test]
fn case5_vec_memory_insert_and_knn() {
    let (_dir, conn) = fresh_db();

    // 插入来源行（facts），再写向量（rowid = facts.id）
    conn.execute("INSERT INTO facts (category, content) VALUES (0, '用户讨厌吃香菜')", [])
        .expect("插入 fact");
    let fact_id: i64 = conn.last_insert_rowid();

    let embedding = embedding_bytes(&vec![0.1_f32; 1536]);
    let source_text = "用户讨厌吃香菜";
    conn.execute(
        "INSERT INTO vec_memory (rowid, embedding, source_text) VALUES (?1, ?2, ?3)",
        rusqlite::params![fact_id, embedding, source_text],
    )
    .expect("插入向量");

    // KNN 查询命中且文本正确
    let query = embedding_bytes(&vec![0.1_f32; 1536]);
    let (hit_rowid, hit_text): (i64, String) = conn
        .query_row(
            "SELECT rowid, source_text FROM vec_memory \
             WHERE embedding MATCH ?1 AND k = 1 \
             ORDER BY distance",
            rusqlite::params![query],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("KNN 命中");
    assert_eq!(hit_rowid, fact_id);
    assert_eq!(hit_text, source_text);
}

/// f32 向量 → sqlite-vec 期望的 blob 字节（little-endian）。
fn embedding_bytes(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|f| f.to_le_bytes()).collect()
}

// ---------- 用例 6：向量级联（应用层联动） ----------

#[test]
fn case6_vector_cascade_with_fact() {
    let (_dir, conn) = fresh_db();

    conn.execute("INSERT INTO facts (category, content) VALUES (1, '昨天去了公园')", [])
        .expect("插入 fact");
    let fact_id: i64 = conn.last_insert_rowid();
    conn.execute(
        "INSERT INTO vec_memory (rowid, embedding, source_text) VALUES (?1, ?2, ?3)",
        rusqlite::params![fact_id, embedding_bytes(&vec![0.2_f32; 1536]), "昨天去了公园"],
    )
    .expect("插入向量");

    // 同事务删除：fact 行 + 对应向量行（B5 repositories 的级联模式预演）
    conn.execute("DELETE FROM vec_memory WHERE rowid = ?1", rusqlite::params![fact_id])
        .expect("删向量");
    conn.execute("DELETE FROM facts WHERE id = ?1", rusqlite::params![fact_id]).expect("删事实");

    let vec_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM vec_memory", [], |row| row.get(0)).expect("查询");
    assert_eq!(vec_count, 0, "向量应随 fact 删除");
}

// ---------- 用例 7：约束 ----------

#[test]
fn case7_constraints() {
    let (_dir, conn) = fresh_db();

    // role 缺省 0
    conn.execute("INSERT INTO messages (session_id, content) VALUES ('s1', 'hi')", [])
        .expect("插入 message（role 缺省）");
    let role: i64 =
        conn.query_row("SELECT role FROM messages", [], |row| row.get(0)).expect("查询");
    assert_eq!(role, 0);

    // summaries.session_id UNIQUE
    conn.execute("INSERT INTO summaries (session_id, summary_text) VALUES ('s1', '摘要')", [])
        .expect("插入 summary");
    let dup =
        conn.execute("INSERT INTO summaries (session_id, summary_text) VALUES ('s1', '重复')", []);
    assert!(dup.is_err(), "session_id 重复应报 UNIQUE");

    // diaries.diary_date UNIQUE
    conn.execute("INSERT INTO diaries (diary_date, content) VALUES ('2026-10-21', '日记')", [])
        .expect("插入 diary");
    let dup =
        conn.execute("INSERT INTO diaries (diary_date, content) VALUES ('2026-10-21', '重复')", []);
    assert!(dup.is_err(), "diary_date 重复应报 UNIQUE");
}

// ---------- 用例 9：勘误 #7 M3 数据迁移（扁平 → 三档，含幂等） ----------

#[test]
fn case9_m3_migrates_legacy_flat_settings_to_tiered() {
    register_vec_extension().expect("vec 注册");
    let dir = tempfile::tempdir().expect("临时目录");
    let db_path = dir.path().join("pet.db");

    // 1) 构造"勘误 #7 前的旧库"：迁移到 v2 后把 type=1 value 改回扁平结构
    {
        let mut conn = Connection::open(&db_path).expect("打开");
        conn.execute_batch(&format!("PRAGMA key = \"x'{TEST_KEY}'\";")).expect("密钥");
        qiqi_lib::db::migrations::chain().to_version(&mut conn, 2).expect("迁移到 v2");
        conn.execute(
            "UPDATE app_settings SET value = '{\"llm_base_url\":\"https://api.deepseek.com/v1\",\"llm_api_key\":\"sk-legacy\",\"llm_model\":\"deepseek-chat\",\"llm_model_custom\":\"\"}' WHERE type = 1",
            [],
        )
        .expect("写回扁平结构");
    }

    // 2) 正式初始化：M3 应把扁平结构升级为三档，且 main 档保留存量配置
    // （锁的借用限制在块内，使后续 drop(db) 合法——E0505 实测坑）
    let value: String = {
        let db = init_with_key(&db_path, TEST_KEY).expect("初始化");
        let conn = db.conn.lock().expect("锁");
        let value: String = conn
            .query_row("SELECT value FROM app_settings WHERE type = 1", [], |row| row.get(0))
            .expect("settings 行");
        let parsed: serde_json::Value = serde_json::from_str(&value).expect("JSON 解析");
        assert_eq!(parsed["main"]["baseUrl"], "https://api.deepseek.com/v1");
        assert_eq!(parsed["main"]["apiKey"], "sk-legacy", "存量 Key 不得丢失");
        assert_eq!(parsed["main"]["model"], "deepseek-chat");
        assert!(parsed.get("memory").is_some(), "memory 档应已建");
        assert!(parsed.get("fallback").is_some(), "fallback 档应已建");
        value
    }; // db 与锁在此释放

    // 3) 幂等：再次初始化不重复改写（main 档配置保持）
    let db2 = init_with_key(&db_path, TEST_KEY).expect("二次初始化");
    let value2: String = {
        let conn2 = db2.conn.lock().expect("锁");
        conn2
            .query_row("SELECT value FROM app_settings WHERE type = 1", [], |row| row.get(0))
            .expect("settings 行")
    };
    assert_eq!(value2, value, "M3 应幂等（重复执行无副作用）");
}

// ---------- 用例 8：密钥不匹配/文件损坏 → 隔离而非崩溃 ----------
#[test]
fn case8_undecryptable_db_is_quarantined() {
    register_vec_extension().expect("vec 注册");
    let dir = tempfile::tempdir().expect("临时目录");
    let db_path = dir.path().join("pet.db");

    // 1) 用密钥 A 正常建库
    let first = init_with_key(&db_path, TEST_KEY).expect("以密钥 A 建库");
    drop(first);

    // 2) 换密钥 B 打开 → 应报 Config 且库被隔离（不静默删除数据）
    let err =
        init_with_key(&db_path, "FEDCBA9876543210FEDCBA9876543210FEDCBA9876543210FEDCBA9876543210")
            .expect_err("密钥不匹配应失败");
    assert!(
        matches!(err, qiqi_lib::error::AppError::Config(_)),
        "应为可行动的 Config 错误: {err:?}"
    );

    // 3) 隔离文件存在（证据保留），原 pet.db 已移走 → 下次启动可重建
    assert!(!db_path.exists(), "损坏库应被移走");
    let quarantined = std::fs::read_dir(dir.path())
        .expect("读取目录")
        .filter_map(|e| e.ok())
        .any(|e| e.file_name().to_string_lossy().starts_with("pet.db.corrupt-"));
    assert!(quarantined, "应存在隔离文件（保留证据）");
}
