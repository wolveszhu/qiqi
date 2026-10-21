//! 常量真源：跨模块共享的配置常量（规范 §4：禁止第二定义）。

/// 向量维度：`vec_memory` 虚拟表与 embedding 输出必须一致（勘误#6）。
/// 对应 OpenAI `text-embedding-3-small`（1536）；变更须重建 vec_memory（B4）。
pub const EMBEDDING_DIM: usize = 1536;

/// 探针轮询间隔（毫秒）：`GetCursorPos` 单点轮询，CPU < 2% 红线的核心参数（C2/E2 实测取舍）。
pub const PROBE_INTERVAL_MS: u64 = 100;

/// SQLCipher 密钥在 Windows 凭据管理器的服务名/用户名（B4）。
pub const KEY_SERVICE: &str = "com.brave.qiqi";
pub const KEY_USER: &str = "sqlcipher-key";

// TODO(C3): LLM 服务商预设常量（必选 DeepSeek/通义千问 + 可选低成本组，
// OpenAI 兼容 base_url + 默认模型 + 用途档位），与「LLM 大模型配置」小节同源。
// TODO(C6): 空闲阈值（30min）、时间窗口锚点（08:30-09:30 / 22:00-23:30）、
// 频率档位区间（1-2/4-6/10+）、工作/游戏进程名单。
