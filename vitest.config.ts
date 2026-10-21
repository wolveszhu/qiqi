import { defineConfig } from "vitest/config";

// Vitest 基建（阶段0/A3）：纯函数单测跑 node 环境；
// 组件挂载测试（F2）引入 @vue/test-utils + happy-dom 后按文件扩展 environment。
export default defineConfig({
  test: {
    environment: "node",
    include: ["src/tests/**/*.spec.ts"],
  },
});
