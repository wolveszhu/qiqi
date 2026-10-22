//! System Prompt 拼装（C4.4，**纯函数**，单测覆盖）。
//!
//! 组成（计划 §二 C4.4）：人设基线 + 关系等级行为指令 + 记忆注入（KNN 命中 +
//! facts 近因）+ 勿扰/情绪基调占位（C6 接入点）+ 禁数值播报规则。
//! 隐私红线：载荷仅含对话文本与本地记忆，**绝不含屏幕内容/进程窗口内容**。

use crate::affection::level::{self, Level};
use crate::config::PERSONA_BASELINE;

/// 混合检索结果（KNN 命中 + facts 近因）。
#[derive(Debug, Default, Clone)]
pub struct RetrievedContext {
    /// 向量检索命中的记忆文本（`vec_memory.source_text`，按相似度排序）。
    pub memory_hits: Vec<String>,
    /// facts 近因条目（content，按 updated_at 倒序）。
    pub facts: Vec<String>,
}

/// 关系等级的最小视图（避免依赖完整 PetStateDto，纯函数易测）。
#[derive(Debug, Clone, Copy)]
pub struct RelationView {
    pub level: i64,
    pub pet_name: &'static str,
}

/// 拼装 System Prompt。
///
/// 等级越界（<1 / >4）夹取到边界指令（不 panic）。
pub fn build_system_prompt(retrieved: &RetrievedContext, relation: &RelationView) -> String {
    let mut prompt = String::from(PERSONA_BASELINE);
    prompt.push_str("\n\n你叫");
    prompt.push_str(relation.pet_name);
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

    // TODO(C6): 勿扰/情绪基调占位（微随机引擎产出后在此注入）
    prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(level: i64) -> RelationView {
        RelationView { level, pet_name: "喵喵" }
    }

    #[test]
    fn prompt_without_memory_contains_baseline_and_rules() {
        let p = build_system_prompt(&RetrievedContext::default(), &view(1));
        assert!(p.contains("七七"), "含人设名");
        assert!(p.contains("不要向用户播报任何数字"), "含禁数值规则");
        assert!(p.contains("刚认识"), "Lv1 指令注入");
        assert!(!p.contains("你记得的"), "无记忆时省略记忆段");
    }

    #[test]
    fn prompt_with_memory_includes_hits() {
        let ctx = RetrievedContext {
            memory_hits: vec!["用户讨厌吃香菜".into()],
            facts: vec!["昨天去了公园".into()],
        };
        let p = build_system_prompt(&ctx, &view(2));
        assert!(p.contains("用户讨厌吃香菜"), "KNN 命中注入");
        assert!(p.contains("昨天去了公园"), "facts 注入");
        assert!(p.contains("熟悉的朋友"), "Lv2 指令注入");
    }

    #[test]
    fn level_changes_directive_and_is_clamped() {
        let low = build_system_prompt(&RetrievedContext::default(), &view(1));
        let high = build_system_prompt(&RetrievedContext::default(), &view(4));
        assert!(low.contains("刚认识"));
        assert!(high.contains("无可替代"));
        // 越界夹取：Lv99 → Lv4 指令；Lv-3 → Lv1 指令（不 panic）
        assert!(build_system_prompt(&RetrievedContext::default(), &view(99)).contains("无可替代"));
        assert!(build_system_prompt(&RetrievedContext::default(), &view(-3)).contains("刚认识"));
    }

    #[test]
    fn prompt_never_contains_numeric_affection() {
        // 人设基线不含任何分数语义（人设红线在 Prompt 层固化）
        let p = build_system_prompt(&RetrievedContext::default(), &view(3));
        assert!(!p.contains("affection"), "不泄露内部字段名");
        assert!(!p.contains("0-1000"), "不播报数值范围");
    }
}
