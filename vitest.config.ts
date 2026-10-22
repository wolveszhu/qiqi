import { defineConfig } from "vitest/config";
import vue from "@vitejs/plugin-vue";

// Vitest 基建（A3 + F2.1）：双环境——
// - 纯函数单测（默认 node）：utils/constants 计算；
// - 组件挂载测试（F2.3）：src/tests/components/** 用 happy-dom（@vue/test-utils 挂载）。
// `.vue` SFC 需 @vitejs/plugin-vue 转换（与 vite.config.ts 同插件）。
// 布局/像素级不在此断言（布局决策已抽纯函数 bubbleLayout/edgeSnap，像素归 F3 手工）。
export default defineConfig({
  plugins: [vue()],
  test: {
    environment: "node",
    environmentMatchGlobs: [["src/tests/components/**", "happy-dom"]],
    include: ["src/tests/**/*.spec.ts"],
  },
});
