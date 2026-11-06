// @vitest-environment happy-dom
/**
 * 窗口 A（PetWindow）右键契约回归：
 * - 右键菜单被拦截 → WebView2 原生菜单（将图像另存为 / 复制图像 / 检查）不再弹出；
 * - 只响应主键：右键不误触发抚摸计分与系统拖拽；
 * - 左键单击 / 双击等原有交互保持不变（本次改动的回归钉子）。
 *
 * 说明：真实 WebView2 菜单由宿主层绘制，happy-dom 无法渲染；
 * 此处断言的是"DOM contextmenu 事件被 preventDefault"这一可观测契约。
 */

import { mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";

import PetWindow from "../../views/pet/PetWindow.vue";

const spies = vi.hoisted(() => ({
  playAction: vi.fn(),
  settleAffection: vi.fn(async () => ({})),
  startDrag: vi.fn(async () => {}),
  show: vi.fn(async () => {}),
  emit: vi.fn(async () => {}),
  setBubbleMode: vi.fn(async () => {}),
}));

vi.mock("@tauri-apps/api/event", () => ({
  emit: spies.emit,
  listen: vi.fn(async () => vi.fn()),
}));

vi.mock("../../composables/usePetRenderer", () => ({
  lookAt: vi.fn(),
  onRendererRebuilt: vi.fn(),
  playAction: spies.playAction,
  setExpression: vi.fn(),
  setIdleMode: vi.fn(),
  setEdgeSleep: vi.fn(),
  usePetRenderer: vi.fn(() => ({})),
}));

vi.mock("../../composables/useProactive", () => ({
  proactiveBubble: { text: "", isStreaming: false },
  useProactive: vi.fn(),
}));

vi.mock("../../composables/useAffection", () => ({ useAffection: vi.fn() }));
vi.mock("../../composables/useSystemProbe", () => ({ useSystemProbe: vi.fn() }));

vi.mock("../../state/petStore", () => ({ petStore: { activeRenderer: "code_cat" } }));
vi.mock("../../state/settingsStore", () => ({
  settingsStore: {
    snapshot: null,
    snapSleepEnabled: true,
    snapShowRatio: 0.5,
    sleepSnap: false,
  },
  applyDndLocal: vi.fn(),
  applySnapshot: vi.fn(),
}));

vi.mock("../../api/pet", () => ({
  settleAffection: spies.settleAffection,
  sleepDecay: vi.fn(async () => {}),
}));
vi.mock("../../api/probe", () => ({ onChatToggle: vi.fn(async () => vi.fn()) }));
vi.mock("../../api/window", () => ({
  getOuterPosition: vi.fn(async () => ({
    x: 0,
    y: 0,
    width: 150,
    height: 150,
    monitor: {},
  })),
  restoreFromSnap: vi.fn(async () => {}),
  setBubbleMode: spies.setBubbleMode,
  setSleepMode: vi.fn(async () => {}),
  show: spies.show,
  snapToEdge: vi.fn(async () => {}),
  startDrag: spies.startDrag,
}));
vi.mock("../../api/settings", () => ({ getTyped: vi.fn(async () => ({})) }));
vi.mock("../../utils/edgeSnap", () => ({ detectSnapEdge: vi.fn(() => null) }));

/** 构造可取消的右键菜单事件（cancelable 才能观察 preventDefault）。 */
function contextMenuEvent(): MouseEvent {
  return new MouseEvent("contextmenu", { bubbles: true, cancelable: true, button: 2 });
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe("PetWindow 右键菜单拦截", () => {
  it("右键点在根元素上 → contextmenu 被 preventDefault", async () => {
    const w = mount(PetWindow);
    await Promise.resolve();
    const e = contextMenuEvent();
    w.get(".pet-root").element.dispatchEvent(e);
    expect(e.defaultPrevented).toBe(true);
    w.unmount();
  });

  it("右键点在 PIXI canvas 等动态子节点上 → 冒泡到根元素同样被拦截", async () => {
    const w = mount(PetWindow);
    await Promise.resolve();
    const canvas = document.createElement("canvas");
    w.get("#pet-canvas").element.appendChild(canvas);
    const e = contextMenuEvent();
    canvas.dispatchEvent(e);
    expect(e.defaultPrevented).toBe(true);
    w.unmount();
  });

  it("兜底：右键点在根元素之外（body）→ document capture 仍拦截", async () => {
    const w = mount(PetWindow);
    await Promise.resolve();
    const e = contextMenuEvent();
    document.body.dispatchEvent(e);
    expect(e.defaultPrevented).toBe(true);
    w.unmount();
  });

  it("卸载后移除兜底监听，不污染其他页面", async () => {
    const w = mount(PetWindow);
    await Promise.resolve();
    w.unmount();
    const e = contextMenuEvent();
    document.body.dispatchEvent(e);
    expect(e.defaultPrevented).toBe(false);
  });
});

describe("PetWindow 主键交互不回归", () => {
  it("左键 pointerdown → 仍触发点击动效与抚摸计分", async () => {
    const w = mount(PetWindow);
    await Promise.resolve();
    w.get(".pet-root").element.dispatchEvent(
      new MouseEvent("pointerdown", { bubbles: true, button: 0 }),
    );
    expect(spies.playAction).toHaveBeenCalledWith("click");
    expect(spies.settleAffection).toHaveBeenCalledWith({ kind: "petStroke" });
    w.unmount();
  });

  it("右键 pointerdown → 不抚摸计分、不启动系统与拖拽", async () => {
    const w = mount(PetWindow);
    await Promise.resolve();
    w.get(".pet-root").element.dispatchEvent(
      new MouseEvent("pointerdown", { bubbles: true, button: 2 }),
    );
    expect(spies.playAction).not.toHaveBeenCalled();
    expect(spies.settleAffection).not.toHaveBeenCalled();
    expect(spies.startDrag).not.toHaveBeenCalled();
    w.unmount();
  });

  it("左键双击 → 仍打开窗口 B 的对话 Tab", async () => {
    const w = mount(PetWindow);
    await Promise.resolve();
    await w.get(".pet-root").trigger("dblclick");
    await Promise.resolve();
    expect(spies.show).toHaveBeenCalledWith("settings");
    expect(spies.emit).toHaveBeenCalledWith("window:chat:open", {});
    w.unmount();
  });
});
