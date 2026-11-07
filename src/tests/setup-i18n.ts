/**
 * Vitest 全局装配：为 @vue/test-utils 的组件挂载注入 vue-i18n 实例。
 *
 * 设置窗口（窗口 B）组件普遍在 setup 内调用 `useI18n()`，若测试挂载时未安装
 * 插件会抛 "Need to install with `app.use` function"。此处统一注入，避免每个
 * spec 重复写 `global.plugins`。
 *
 * 默认语言保持 `zh-CN`，与线上缺省一致——既有断言文案的中文字符串无需改动。
 * 需验证英文文案的用例可在单个测试内 `i18n.global.locale.value = "en-US"`。
 */

import { config } from "@vue/test-utils";

import i18n from "../i18n";

config.global.plugins = [i18n];
