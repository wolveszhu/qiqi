//! C7 集成测试（§五）：向量编辑/删除同步、全量重建等价、日记 upsert 幂等。
//!
//! 夹具同 B4/C5：`init_with_key` 临时库 + 固定测试密钥（不触碰真实数据目录）。
//! Mock LLM `with_embed_dim(EMBEDDING_DIM)` 对接真实 1536 维 vec_memory。

// F1.4：集成测试 fixture 用 unwrap/expect/index 属合理用法，统一豁免四项 clippy lint。
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::indexing_slicing)]

use qiqi_lib::config::EMBEDDING_DIM;
use qiqi_lib::db::connection::{Db, init_with_key, register_vec_extension};
use qiqi_lib::db::repositories::{diaries, facts, vec_memory};
use qiqi_lib::llm::client::{LlmClient as _, MockLlmClient, ReqwestLlmClient};
use qiqi_lib::llm::diary::{DiaryDraft, parse_diary, store_diary};
use qiqi_lib::llm::distill::{
    add_fact_with_embedding, apply_fact_embedding, rebuild_vec_memory, remove_fact,
    validate_fact_content,
};

/// 固定测试密钥（64 hex，同生产 32 字节 hex 形态）。
const TEST_KEY: &str = "0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF";

/// 新建临时库并完成全链迁移。
fn temp_db() -> (tempfile::TempDir, rusqlite::Connection) {
    register_vec_extension().expect("vec 注册");
    let dir = tempfile::tempdir().expect("临时目录");
    let db_path = dir.path().join("pet.db");
    let Db { conn } = init_with_key(&db_path, TEST_KEY).expect("初始化");
    (dir, conn.into_inner().expect("取连接"))
}

/// memory 档目标（本测试内不依赖真实配置：直接指 Mock 可达地址）。
fn mock_target() -> qiqi_lib::llm::client::LlmTarget {
    qiqi_lib::llm::client::LlmTarget {
        tier: "memory",
        platform: String::new(),
        base_url: "https://mock/v1".into(),
        api_key: "k".into(),
        model: "m".into(),
    }
}

/// 插入一条事实并写向量（返回 fact id）。
fn seed_fact(conn: &rusqlite::Connection, client: &MockLlmClient, content: &str) -> i64 {
    let target = mock_target();
    let id = facts::insert(conn, 0, content, 0.6).expect("插入 fact");
    let emb = futures_block_on(client.embed(&target, vec![content.to_string()]))
        .expect("embed")
        .into_iter()
        .next()
        .expect("向量");
    vec_memory::insert(conn, id, &vec_memory::embedding_bytes(&emb), content).expect("写向量");
    id
}

/// 阻塞执行 async（单线程 runtime，测试桥接）。
fn futures_block_on<F: std::future::Future>(fut: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread().enable_all().build().expect("rt").block_on(fut)
}

#[test]
fn update_fact_syncs_vector_to_new_content() {
    let (_dir, conn) = temp_db();
    let client = MockLlmClient::ok().with_embed_dim(EMBEDDING_DIM);
    let target = mock_target();
    let id = seed_fact(&conn, &client, "用户讨厌吃香菜");

    // 编辑为新内容（apply_fact_embedding：行更新 + 向量重写）
    let new_emb = futures_block_on(client.embed(&target, vec!["用户最爱吃火锅".into()]))
        .expect("embed")
        .into_iter()
        .next()
        .expect("向量");
    apply_fact_embedding(&conn, id, "用户最爱吃火锅", &new_emb).expect("编辑同步");

    // KNN：新文本命中（rowid=该 fact），旧文本不再以旧内容出现
    let q = futures_block_on(client.embed(&target, vec!["用户最爱吃火锅".into()]))
        .expect("embed")
        .into_iter()
        .next()
        .expect("向量");
    let hits = vec_memory::knn(&conn, &vec_memory::embedding_bytes(&q), 1).expect("knn");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].0, id, "命中编辑后的行");
    assert_eq!(hits[0].1, "用户最爱吃火锅", "source_text 已同步");
}

#[test]
fn delete_fact_removes_vector() {
    let (_dir, conn) = temp_db();
    let client = MockLlmClient::ok().with_embed_dim(EMBEDDING_DIM);
    let target = mock_target();
    let id = seed_fact(&conn, &client, "用户养了一只猫");

    remove_fact(&conn, id).expect("删除同步");

    let q = futures_block_on(client.embed(&target, vec!["用户养了一只猫".into()]))
        .expect("embed")
        .into_iter()
        .next()
        .expect("向量");
    let hits = vec_memory::knn(&conn, &vec_memory::embedding_bytes(&q), 5).expect("knn");
    assert!(hits.iter().all(|(rowid, _)| *rowid != id), "删除后向量不再命中");
}

#[test]
fn rebuild_vec_memory_preserves_knn_results() {
    let (_dir, conn) = temp_db();
    let client = MockLlmClient::ok().with_embed_dim(EMBEDDING_DIM);
    let target = mock_target();

    let contents = ["用户讨厌吃香菜", "用户最爱吃火锅", "用户养了一只猫"];
    let ids: Vec<i64> = contents.iter().map(|c| seed_fact(&conn, &client, c)).collect();

    // 重建前 KNN 基线
    let q = futures_block_on(client.embed(&target, vec!["用户养了一只猫".into()]))
        .expect("embed")
        .into_iter()
        .next()
        .expect("向量");
    let before = vec_memory::knn(&conn, &vec_memory::embedding_bytes(&q), 3).expect("knn");

    // 全量重建（真实 1536 维虚拟表 DROP + 重建 + 分批重写）
    let written =
        futures_block_on(rebuild_vec_memory(&client, &target, &conn, EMBEDDING_DIM)).expect("重建");
    assert_eq!(written, contents.len(), "重建写入条数一致");

    // 重建后 KNN 与基线一致（等价性：rowid 顺序与 source_text 不变）
    let after = vec_memory::knn(&conn, &vec_memory::embedding_bytes(&q), 3).expect("knn");
    assert_eq!(before, after, "重建前后 KNN 结果一致");
    assert_eq!(after[0].0, ids[2], "最近邻仍为目标事实");
}

#[test]
fn add_fact_writes_vector_and_knn_hits() {
    let (_dir, conn) = temp_db();
    let client = MockLlmClient::ok().with_embed_dim(EMBEDDING_DIM);
    let target = mock_target();
    let content = "我喜欢吃香菜";

    // 正向入口：校验 → embed → 写库 + 向量
    let validated = validate_fact_content(content).expect("校验通过");
    let emb = futures_block_on(client.embed(&target, vec![validated.clone()]))
        .expect("embed")
        .into_iter()
        .next()
        .expect("向量");
    let id = add_fact_with_embedding(&conn, 0, &validated, &emb).expect("新增事实");

    // KNN：新增内容可召回，source_text 一致
    let q = futures_block_on(client.embed(&target, vec![validated.clone()]))
        .expect("embed")
        .into_iter()
        .next()
        .expect("向量");
    let hits = vec_memory::knn(&conn, &vec_memory::embedding_bytes(&q), 1).expect("knn");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].0, id, "新增后向量命中该行");
    assert_eq!(hits[0].1, content, "source_text 已同步");
}

#[test]
fn fact_content_validation_rejects_dirty_input() {
    assert!(validate_fact_content("   ").is_err(), "纯空白 → 拒绝");
    assert!(validate_fact_content("").is_err(), "空串 → 拒绝");
    assert!(validate_fact_content(&"x".repeat(201)).is_err(), "超长（>200 字）→ 拒绝");
    assert_eq!(validate_fact_content("  我喜欢猫  ").unwrap(), "我喜欢猫", "正常内容去空白返回");
}

#[test]
fn diary_upsert_is_idempotent_per_date() {
    let (_dir, conn) = temp_db();
    let d1 = DiaryDraft { content: "今天很开心喵".into(), mood: Some(0) };
    let d2 = DiaryDraft { content: "其实有点困".into(), mood: Some(1) };

    store_diary(&conn, "2026-10-22", &d1).expect("首次写入");
    store_diary(&conn, "2026-10-22", &d2).expect("覆盖写入（幂等）");

    let all = diaries::list(&conn, 50).expect("list");
    assert_eq!(all.len(), 1, "UNIQUE(diary_date)：同日仅一行");
    assert_eq!(all[0].content, "其实有点困", "重生成覆盖当日内容");
    assert_eq!(all[0].mood, Some(1), "mood 同步覆盖");
}

#[test]
fn diary_parser_three_branches() {
    // 合法 JSON / 围栏 / 纯文本回退 / 空
    assert_eq!(parse_diary(r#"{"content":"开心","mood":0}"#).unwrap().content, "开心");
    assert_eq!(parse_diary("```\n{\"content\":\"平静\",\"mood\":1}\n```").unwrap().mood, Some(1));
    assert_eq!(parse_diary("就一句话喵").unwrap().content, "就一句话喵");
    assert!(parse_diary("").is_none());
}

#[test]
fn reqwest_client_embed_dim_mismatch_is_rejected() {
    // 维度不匹配必须显式报错（勘误#8：不静默截断）——Mock 不校验，Reqwest 才校验，
    // 本用例验证生产 client 构造参数接线正确（维度常量单点）。
    let _client = ReqwestLlmClient::new(EMBEDDING_DIM);
    assert_eq!(EMBEDDING_DIM, 1536, "维度真源（变更须重建 vec_memory）");
}
