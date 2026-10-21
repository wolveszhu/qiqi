-- 依据：docs/design/桌宠.md §5.1；勘误决策见父计划附录 A §A.1（#1–#4）
-- 单事务幂等：IF NOT EXISTS 建表 + NOT EXISTS 守卫种子（rusqlite_migration 每迁移独立事务）

-- 1. messages（对话记录）
CREATE TABLE IF NOT EXISTS messages (
    id          INTEGER     PRIMARY KEY AUTOINCREMENT, -- 勘误#1：BIGINTEGER 规范化为 INTEGER
    session_id  VARCHAR(50) NOT NULL,
    role        TINYINT     NOT NULL DEFAULT 0,        -- 0=user, 1=assistant（勘误#2）
    content     TEXT        NOT NULL,
    tokens      INTEGER     DEFAULT 0,
    created_at  TIMESTAMP   DEFAULT CURRENT_TIMESTAMP,
    updated_at  TIMESTAMP   DEFAULT CURRENT_TIMESTAMP
);

-- 2. summaries（会话摘要）
CREATE TABLE IF NOT EXISTS summaries (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id   VARCHAR(50) UNIQUE NOT NULL,          -- 文档约束：UNIQUE, NOT NULL
    summary_text TEXT        NOT NULL,
    created_at   TIMESTAMP   DEFAULT CURRENT_TIMESTAMP,
    updated_at   TIMESTAMP   DEFAULT CURRENT_TIMESTAMP
);

-- 3. facts（结构化事实）
CREATE TABLE IF NOT EXISTS facts (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    category   TINYINT NOT NULL,                       -- 0=preference, 1=event, 2=profile
    content    TEXT    NOT NULL,                       -- 如 '用户讨厌吃香菜'
    confidence REAL    DEFAULT 1.0,                    -- 置信度 0-1，防 AI 幻觉
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 4. pet_states（桌宠状态；单用户单角色，id 固定 1，无 AUTOINCREMENT）
CREATE TABLE IF NOT EXISTS pet_states (
    id                 INTEGER PRIMARY KEY,
    pet_name           VARCHAR(20) DEFAULT '喵喵',
    relationship_level INTEGER     DEFAULT 1,        -- 1陌生 2熟人 3亲密 4羁绊
    affection_points   INTEGER     DEFAULT 0,        -- 0-1000
    last_decay_at      DATETIME,                     -- NULL 允许；衰减结算锚点
    active_renderer    TEXT        DEFAULT 'code_cat',
    renderer_config    JSON,                         -- NULL 允许
    last_interaction   TIMESTAMP                     -- NULL 允许；空闲计算用
);

-- 5. affection_logs（好感度流水）
CREATE TABLE IF NOT EXISTS affection_logs (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    change_value INTEGER NOT NULL,                   -- 正加负减
    reason       TEXT    NOT NULL,                   -- 'daily_interaction'/'long_absence'/'chat_reward'/'taboo_violation'…
    created_at   TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 6. app_settings（全局配置；勘误#3：以字段清单为准，不建 key 列）
CREATE TABLE IF NOT EXISTS app_settings (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    type                 INTEGER DEFAULT 1,          -- 1=大模型, 2=勿扰时段
    is_custom_quota_used INTEGER DEFAULT 1,          -- 1=使用内置额度, 2=不使用
    value                JSON                        -- NULL 允许；JSON 字符串（llm_* / dnd_*）
);

-- 7. diaries（日记）
CREATE TABLE IF NOT EXISTS diaries (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    diary_date DATE UNIQUE NOT NULL,                 -- 文档约束：UNIQUE, NOT NULL（每日一篇）
    content    TEXT    NOT NULL,                     -- 第一人称日记正文
    mood       INTEGER,                              -- 当日情绪（可空）
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 补充索引（普通索引，服务 §5.2 查询路径，非文档原文，附录 A §A.4）
CREATE INDEX IF NOT EXISTS idx_messages_session_time ON messages (session_id, created_at);
CREATE INDEX IF NOT EXISTS idx_facts_category        ON facts (category);
CREATE INDEX IF NOT EXISTS idx_aff_logs_time         ON affection_logs (created_at);
CREATE INDEX IF NOT EXISTS idx_app_settings_type     ON app_settings (type);

-- 种子数据（幂等：NOT EXISTS 守卫）
INSERT INTO pet_states (id)
SELECT 1 WHERE NOT EXISTS (SELECT 1 FROM pet_states WHERE id = 1);

INSERT INTO app_settings (type, value)
SELECT 1, '{"main":{"baseUrl":"https://api.deepseek.com/v1","model":"deepseek-chat","apiKey":"","modelCustom":""},"memory":{"baseUrl":"https://dashscope.aliyuncs.com/compatible-mode/v1","model":"qwen-flash","apiKey":"","modelCustom":""},"fallback":{"baseUrl":"https://open.bigmodel.cn/api/paas/v4","model":"glm-4-flash","apiKey":"","modelCustom":""}}'
WHERE NOT EXISTS (SELECT 1 FROM app_settings WHERE type = 1);

INSERT INTO app_settings (type, value)
SELECT 2, '{"dnd_enabled":false,"dnd_start_time":"09:00","dnd_end_time":"18:00"}'
WHERE NOT EXISTS (SELECT 1 FROM app_settings WHERE type = 2);
