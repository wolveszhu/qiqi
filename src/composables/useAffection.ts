/**
 * 好感度事件消费（C5.6 实装）：affection:changed → petStore 同步 + 表情联动。
 *
 * 纪律（禁数值红线）：窗口 A **只取 mood** 驱动表情；`delta/newPoints/newLevel`
 * 绝不渲染到窗口 A（仅进窗口 B 看板 D2）。数值变化一律转成情绪表达。
 */

import { onMounted, onUnmounted } from "vue";

import { listen } from "@tauri-apps/api/event";

import { getState, settleAffection } from "../api/pet";
import { applyPetState, petStore } from "../state/petStore";
import { setExpression } from "./usePetRenderer";
import type { AffectionChangedPayload } from "../api/types";
import type { UnlistenFn } from "@tauri-apps/api/event";

const unlisteners: UnlistenFn[] = [];

export function useAffection(): void {
  // 每日首次互动打卡（PetWindow mount 触发；结算引擎保证当日唯一 +10）
  // 成功后拉最新状态刷新 lastInteraction 镜像（C6 傲娇判定数据源保持新鲜）
  onMounted((): void => {
    void settleAffection({ kind: "checkIn" })
      .then(() => getState())
      .then(applyPetState)
      .catch((e: unknown) => {
        console.warn("[affection] 打卡结算失败", e);
      });
  });

  void (async (): Promise<void> => {
    try {
      unlisteners.push(
        await listen<AffectionChangedPayload>("affection:changed", (event) => {
          const p = event.payload;
          // 数值仅进 petStore 镜像（供 D2 看板读取）——窗口 A 不渲染
          petStore.relationshipLevel = p.newLevel;
          petStore.affectionPoints = p.newPoints;
          // 表情联动：只取 mood（happy/angry/neutral）
          void setExpression(p.mood);
        }),
      );
    } catch (e) {
      console.warn("[affection] 事件订阅失败", e);
    }
  })();

  onUnmounted((): void => {
    for (const un of unlisteners.splice(0)) un();
  });
}
