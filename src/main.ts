/**
 * 应用入口：按 Tauri 窗口 label 分发挂载（规范 §3 / B1）。
 *
 * - label === "settings" → SettingsWindow（设置层）
 * - 其余（含浏览器直开兜底）→ PetWindow（桌宠层）
 */

import { createApp } from "vue";

import PetWindow from "./views/pet/PetWindow.vue";
import SettingsWindow from "./views/settings/SettingsWindow.vue";
import "./styles/tokens.css";
import "./styles/base.css";

/** 解析当前窗口 label；非 Tauri 环境（浏览器直开）兜底为桌宠层。 */
async function resolveRootComponent(): Promise<typeof PetWindow | typeof SettingsWindow> {
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    return getCurrentWindow().label === "settings" ? SettingsWindow : PetWindow;
  } catch {
    return PetWindow;
  }
}

void resolveRootComponent().then((Root) => {
  // 桌宠层（窗口 A）整窗透明：body 默认背景（--bg-page）只属于设置层，
  // 否则 400×400 不透明色块盖住桌面、透明窗口失效（B1）。
  if (Root === PetWindow) {
    document.body.classList.add("pet-window");
  }
  createApp(Root).mount("#app");
});
