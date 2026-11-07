/**
 * 应用入口：按 Tauri 窗口 label 分发挂载（规范 §3 / B1）。
 *
 * - label === "settings" → SettingsWindow（设置层）
 * - 其余（含浏览器直开兜底）→ PetWindow（桌宠层）
 */

import { createApp } from "vue";

import PetWindow from "./views/pet/PetWindow.vue";
import SettingsWindow from "./views/settings/SettingsWindow.vue";
import i18n from "./i18n";
import { loadLocale } from "./composables/useLocale";
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

void resolveRootComponent().then(async (Root) => {
  // 桌宠层（窗口 A）整窗透明：body 默认背景（--bg-page）只属于设置层，
  // 否则 400×400 不透明色块盖住桌面、透明窗口失效（B1）。
  if (Root === PetWindow) {
    // html 也加：背景会传播到视口画布，只清 body 仍可能露出方形底块
    document.body.classList.add("pet-window");
    document.documentElement.classList.add("pet-window");
  }
  // 启动加载语言偏好（设置窗口 UI 与托盘菜单据此渲染）；失败不阻断启动
  await loadLocale().catch(() => {});
  createApp(Root).use(i18n).mount("#app");
});
