/**
 * 渲染器契约（baseline §D / 接口契约 §5）：桌宠形象与物理交互的唯一抽象。
 *
 * Phase 3 预留：`CustomSpriteRenderer` 实现同一接口，经 factory 注册即可替换（G4 接入指南）。
 * 约束（code-standards §6 / 规范 §3）：实现类零 Vue 依赖、PIXI 实例普通类字段持有、
 * 单一 Ticker、帧内零 new、卸载必须 destroy(true)。
 */

/**
 * 动作名（值域真源：`docs/design/接口契约.md` §5）。
 * click = 点击果冻冲击（C1 交互核心）；drag/stretch/sleep 供 C2/C6 使用；eat 预留 Phase 2。
 */
import type { SnapEdge } from "../utils/edgeSnap";

export type PetAction = "idle" | "click" | "drag" | "sleep" | "stretch" | "eat";

/** 表情名（C1 最小集 + 情绪反馈；affection:changed 驱动 happy/angry；C6 傲娇 tsundere）。 */
export type PetExpression = "neutral" | "happy" | "angry" | "sleepy" | "tsundere";

/** 角色包围盒（屏幕坐标，供 C2 命中判定/穿透）。 */
export interface HitRegion {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface IPetRenderer {
  /** 挂载到容器（PIXI Application 初始化 + 首帧绘制）。 */
  mount(container: HTMLElement): Promise<void>;
  /** 播放一次性动作（拖拽/睡/醒等；由 playAction 调用方驱动）。 */
  playAction(name: PetAction): void;
  /** 设置持续表情（聊天流/好感度事件驱动）。 */
  setExpression(name: PetExpression): void;
  /** 视线跟随（窗口内坐标，限幅在 physics.eyeOffset 内）。 */
  lookAt(x: number, y: number): void;
  /** 当前角色包围盒（C2 每帧/变化时读取上报）。 */
  getHitRegion(): HitRegion;
  /** 空闲降帧开关（E2 待机 30fps 优化，勘误#18）。 */
  setIdleMode?(enabled: boolean): void;
  /**
   * 边缘吸附休眠视觉（D4.4）：形象整体旋转 90° 躺下（左=顺时针 / 右=逆时针），
   * 按 `showRatio` 决定露出圆盘的比例，命中区随之切换；false 恢复原状。
   */
  setEdgeSleep?(
    sleeping: boolean,
    side?: SnapEdge | null,
    showRatio?: number,
  ): void;
  /** 卸载：destroy(true) + 移除监听（usePetRenderer onUnmounted 调用）。 */
  dispose(): void;
}
