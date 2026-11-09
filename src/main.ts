/**
 * 应用入口：按 Tauri 窗口 label 分发挂载（规范 §3 / B1）。
 *
 * - label === "settings" → SettingsWindow（设置层）
 * - label === "translator" → TranslatorWindow（划词翻译浮窗）
 * - 其余（含浏览器直开兜底）→ PetWindow（桌宠层）
 */

import { createApp } from "vue";

import PetWindow from "./views/pet/PetWindow.vue";
import SettingsWindow from "./views/settings/SettingsWindow.vue";
import TranslatorWindow from "./views/translate/TranslatorWindow.vue";
import i18n from "./i18n";
import { loadLocale } from "./composables/useLocale";
import "./styles/tokens.css";
import "./styles/base.css";

/** 解析当前窗口 label；非 Tauri 环境（浏览器直开）兜底为桌宠层。 */
async function resolveRootComponent(): Promise<
  typeof PetWindow | typeof SettingsWindow | typeof TranslatorWindow
> {
  try {
    const { getCurrentWindow } = await import("@tauri-apps/api/window");
    const label = getCurrentWindow().label;
    if (label === "settings") return SettingsWindow;
    if (label === "translator") return TranslatorWindow;
    return PetWindow;
  } catch {
    return PetWindow;
  }
}

void resolveRootComponent().then(async (Root) => {
  // 桌宠层与划词翻译浮窗均为整窗透明：body 默认背景（--bg-page）只属于设置层，
  // 否则会露出不透明色块盖住桌面（B1）。
  if (Root === PetWindow || Root === TranslatorWindow) {
    document.body.classList.add("pet-window");
    document.documentElement.classList.add("pet-window");
  }
  // 启动加载语言偏好（设置窗口 UI 与托盘菜单据此渲染）；失败不阻断启动
  await loadLocale().catch(() => {});
  createApp(Root).use(i18n).mount("#app");
});
