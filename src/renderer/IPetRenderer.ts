/**
 * 渲染器契约（baseline §D / 接口契约 §5）：桌宠形象与物理交互的唯一抽象。
 *
 * Phase 3 预留：`CustomSpriteRenderer` 实现同一接口，经 factory 注册即可替换（G4 接入指南）。
 * 约束（code-standards §6 / 规范 §3）：实现类零 Vue 依赖、PIXI 实例普通类字段持有、
 * 单一 Ticker、帧内零 new、卸载必须 destroy(true)。
 */

/** 动作名（C1 最小集 + 模块 6 睡/醒；后续 playAction 调用方只允许用这些名）。 */
export type PetAction = "idle" | "drag" | "sleep" | "stretch" | "eat";

/** 表情名（C1 最小集 + 情绪反馈；affection:changed 驱动 happy/angry）。 */
export type PetExpression = "neutral" | "happy" | "angry" | "sleepy";

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
  /** 卸载：destroy(true) + 移除监听（usePetRenderer onUnmounted 调用）。 */
  dispose(): void;
}
