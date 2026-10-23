//! System Prompt 拼装（C4.4，**纯函数**，单测覆盖）。
//!
//! 组成（计划 §二 C4.4）：人设基线 + 关系等级行为指令 + 记忆注入（KNN 命中 +
//! facts 近因）+ 勿扰/情绪基调占位（C6 接入点）+ 禁数值播报规则。
//! 隐私红线：载荷仅含对话文本与本地记忆，**绝不含屏幕内容/进程窗口内容**。

use crate::affection::level::{self, Level};
use crate::config::PERSONA_BASELINE;
use chrono::TimeZone;

/// 混合检索结果（KNN 命中 + facts 近因）。
#[derive(Debug, Default, Clone)]
pub struct RetrievedContext {
    /// 向量检索命中的记忆文本（`vec_memory.source_text`，按相似度排序）。
    pub memory_hits: Vec<String>,
    /// facts 近因条目（content，按 updated_at 倒序）。
    pub facts: Vec<String>,
}

/// 关系等级的最小视图（避免依赖完整 PetStateDto，纯函数易测）。
#[derive(Debug, Clone)]
pub struct RelationView {
    pub level: i64,
    /// 宠物名（来自 `pet_states.pet_name`，用户可改；不再硬编码常量，B 阶段）。
    pub pet_name: String,
}

/// 对话基调上下文（C6：主动互动/情绪状态注入点）。
#[derive(Debug, Clone, Copy, Default)]
pub struct ConversationCtx {
    /// 傲娇：距上次互动 ≥ `TSUNDERE_DAYS` 天 → true（连 N 天没理它，有点小别扭）。
    pub tsundere: bool,
    /// 慢回复：上一轮响应 > `SLOW_REPLY_THRESHOLD_MS` → true（E3.5 轻量情绪反馈：
    /// 下一句自然带一句"刚卡了一下"式歉意，但不反复道歉）。
    pub slow_reply: bool,
}

/// 傲娇判定阈值（天）：距上次互动达到此值注入傲娇/幽怨基调（C6.6）。
pub const TSUNDERE_DAYS: i64 = 3;

/// 慢回复阈值（毫秒）：单轮响应超此值视为"卡顿"，下一轮注入轻量歉意基调（E3.5）。
pub const SLOW_REPLY_THRESHOLD_MS: u64 = 8_000;

/// 纯函数：距上次互动是否达傲娇阈值（无记录/解析失败 → false；不 panic）。
pub fn is_tsundere(last_interaction: Option<&str>, now: chrono::DateTime<chrono::Local>) -> bool {
    let Some(s) = last_interaction else { return false };
    let Ok(last) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") else {
        return false;
    };
    let last = chrono::Local.from_local_datetime(&last).single().unwrap_or_else(chrono::Local::now);
    (now - last).num_days() >= TSUNDERE_DAYS
}

/// 拼装 System Prompt。
///
/// 等级越界（<1 / >4）夹取到边界指令（不 panic）；`ctx.tsundere` 注入傲娇基调。
pub fn build_system_prompt(
    retrieved: &RetrievedContext,
    relation: &RelationView,
    ctx: &ConversationCtx,
) -> String {
    let mut prompt = String::from(PERSONA_BASELINE);
    prompt.push_str("\n\n你叫");
    prompt.push_str(&relation.pet_name);
    prompt.push('。');

    // 关系等级行为指令（夹取 1..=4，由 C5 level 模块单一真源提供）
    let lvl = match relation.level.clamp(1, 4) {
        1 => Level::L1,
        2 => Level::L2,
        3 => Level::L3,
        _ => Level::L4,
    };
    prompt.push_str(level::behavior_prompt(lvl));
    prompt.push_str("\n\n");

    // 傲娇基调（C6.6）：连数日未互动 → 假装不在意但有点小别扭
    if ctx.tsundere {
        prompt.push_str(
            "（情绪：你有点傲娇——其实很在意对方，但嘴上不承认，语气带点小别扭和幽怨，\
             偶尔用\"你还记得回来啊\"之类的话，不过很快又黏上去）\n\n",
        );
    }

    // 慢回复轻量歉意（E3.5）：上一轮卡顿 → 自然带一句"刚卡了一下"，但不反复道歉
    if ctx.slow_reply {
        prompt.push_str(
            "（情绪：上一句回得有点慢，你有点小歉意，这一句自然地提一句\"抱歉喵，刚卡了一下\"\
             之类的话就好，别一直道歉，说完立刻恢复活泼）\n\n",
        );
    }

    // 记忆注入（有则注入，无则省略段落——保持 Prompt 精简）
    if !retrieved.memory_hits.is_empty() {
        prompt.push_str("【你记得的关于用户的事】\n");
        for hit in &retrieved.memory_hits {
            prompt.push_str("- ");
            prompt.push_str(hit);
            prompt.push('\n');
        }
        prompt.push('\n');
    }
    if !retrieved.facts.is_empty() {
        prompt.push_str("【最近了解到的用户特点】\n");
        for fact in &retrieved.facts {
            prompt.push_str("- ");
            prompt.push_str(fact);
            prompt.push('\n');
        }
        prompt.push('\n');
    }

    prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(level: i64) -> RelationView {
        // 测试虚拟名（非七七/非喵喵），验证名字来自参数而非硬编码常量
        RelationView { level, pet_name: "团子".to_string() }
    }

    #[test]
    fn prompt_without_memory_contains_baseline_and_rules() {
        let p = build_system_prompt(
            &RetrievedContext::default(),
            &view(1),
            &ConversationCtx::default(),
        );
        assert!(p.contains("你叫团子"), "名字来自参数（库 pet_name），非写死常量");
        assert!(p.contains("不要向用户播报任何数字"), "含禁数值规则");
        assert!(p.contains("刚认识"), "Lv1 指令注入");
        assert!(!p.contains("你记得的"), "无记忆时省略记忆段");
        assert!(!p.contains("七七"), "不得残留旧硬编码七七");
        assert!(!p.contains("喵喵"), "不得残留旧硬编码喵喵");
    }

    #[test]
    fn prompt_with_memory_includes_hits() {
        let ctx = RetrievedContext {
            memory_hits: vec!["用户讨厌吃香菜".into()],
            facts: vec!["昨天去了公园".into()],
        };
        let p = build_system_prompt(&ctx, &view(2), &ConversationCtx::default());
        assert!(p.contains("用户讨厌吃香菜"), "KNN 命中注入");
        assert!(p.contains("昨天去了公园"), "facts 注入");
        assert!(p.contains("熟悉的朋友"), "Lv2 指令注入");
    }

    #[test]
    fn level_changes_directive_and_is_clamped() {
        let def = ConversationCtx::default();
        let low = build_system_prompt(&RetrievedContext::default(), &view(1), &def);
        let high = build_system_prompt(&RetrievedContext::default(), &view(4), &def);
        assert!(low.contains("刚认识"));
        assert!(high.contains("无可替代"));
        // 越界夹取：Lv99 → Lv4 指令；Lv-3 → Lv1 指令（不 panic）
        assert!(
            build_system_prompt(&RetrievedContext::default(), &view(99), &def).contains("无可替代")
        );
        assert!(
            build_system_prompt(&RetrievedContext::default(), &view(-3), &def).contains("刚认识")
        );
    }

    #[test]
    fn prompt_never_contains_numeric_affection() {
        // 人设基线不含任何分数语义（人设红线在 Prompt 层固化）
        let p = build_system_prompt(
            &RetrievedContext::default(),
            &view(3),
            &ConversationCtx::default(),
        );
        assert!(!p.contains("affection"), "不泄露内部字段名");
        assert!(!p.contains("0-1000"), "不播报数值范围");
    }

    #[test]
    fn tsundere_directive_injected_only_when_flagged() {
        let base = build_system_prompt(
            &RetrievedContext::default(),
            &view(2),
            &ConversationCtx::default(),
        );
        assert!(!base.contains("傲娇"), "默认无傲娇基调");
        let tsu = build_system_prompt(
            &RetrievedContext::default(),
            &view(2),
            &ConversationCtx { tsundere: true, slow_reply: false },
        );
        assert!(tsu.contains("傲娇"), "tsundere=true 注入傲娇基调");
    }

    #[test]
    fn slow_reply_directive_injected_only_when_flagged() {
        let base = build_system_prompt(
            &RetrievedContext::default(),
            &view(2),
            &ConversationCtx::default(),
        );
        assert!(!base.contains("卡了一下"), "默认无慢回复歉意");
        let slow = build_system_prompt(
            &RetrievedContext::default(),
            &view(2),
            &ConversationCtx { tsundere: false, slow_reply: true },
        );
        assert!(slow.contains("卡了一下"), "slow_reply=true 注入轻量歉意基调");
    }

    #[test]
    fn persona_name_comes_from_state_not_hardcoded() {
        // B 阶段：名字完全来自 pet_name 参数（库值），无硬编码常量；
        // 基线已不含名字，仅"你叫X"处注入，天然一致。
        let p = build_system_prompt(
            &RetrievedContext::default(),
            &view(1),
            &ConversationCtx::default(),
        );
        assert!(p.contains("你叫团子"), "名字来自参数（库 pet_name）");
        assert!(!p.contains("七七"), "不得残留旧硬编码七七");
        assert!(!p.contains("喵喵"), "不得残留旧硬编码喵喵");
    }

    #[test]
    fn is_tsundere_threshold_at_three_days() {
        let now = chrono::Local::now();
        let day = chrono::Duration::days(1);
        let fmt = |d: chrono::DateTime<chrono::Local>| d.format("%Y-%m-%d %H:%M:%S").to_string();
        assert!(!is_tsundere(None, now), "无记录 → false");
        // 2 天前：未达阈值
        let two = now - day * 2;
        assert!(!is_tsundere(Some(&fmt(two)), now), "2 天未达阈值");
        // 恰好 3 天前：达阈值
        let three = now - day * 3;
        assert!(is_tsundere(Some(&fmt(three)), now), "3 天达阈值");
        // 5 天前：达阈值
        let five = now - day * 5;
        assert!(is_tsundere(Some(&fmt(five)), now), "5 天达阈值");
    }
}
