import DefaultTheme from "vitepress/theme";
import type { Theme } from "vitepress";
import "./style.css";
import Hero from "./components/Hero.vue";
import FeatureGrid from "./components/FeatureGrid.vue";
import DownloadBox from "./components/DownloadBox.vue";

// 官网自定义主题入口：仅引入本站样式与零耦合组件，不引用应用任何模块。
export default {
  extends: DefaultTheme,
  enhanceApp({ app }) {
    app.component("Hero", Hero);
    app.component("FeatureGrid", FeatureGrid);
    app.component("DownloadBox", DownloadBox);
  },
} satisfies Theme;
