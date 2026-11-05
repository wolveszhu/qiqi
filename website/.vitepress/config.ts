import { defineConfig } from "vitepress";

// 站点配置：中英双语，默认中文（root = zh），英文在 /en/。
// 所有自定义代码仅位于 website/，对应用零依赖。
const repo = "https://github.com/wolveszhu/qiqi";

export default defineConfig({
  srcDir: "docs",
  title: "七七 · 桌面数字生命",
  description: "住在你桌面上的数字生命：有性格、有记忆、有主动性，会主动找你说话。",
  lastUpdated: true,
  cleanUrls: true,
  appearance: false, // 果冻奶油风为浅色专属设计，关闭深浅切换
  vite: { build: { emptyOutDir: false } },
  head: [
    ["link", { rel: "icon", type: "image/png", href: "/logo.png" }],
    ["link", { rel: "preconnect", href: "https://fonts.googleapis.com" }],
    ["link", { rel: "preconnect", href: "https://fonts.gstatic.com", crossorigin: "" }],
    [
      "link",
      {
        href: "https://fonts.googleapis.com/css2?family=ZCOOL+QingKe+HuangYou&family=Noto+Sans+SC:wght@400;500;700;900&display=swap",
        rel: "stylesheet",
      },
    ],
    ["meta", { property: "og:title", content: "七七 · 桌面数字生命" }],
    [
      "meta",
      {
        property: "og:description",
        content: "住在你桌面上的数字生命：有性格、有记忆、有主动性，会主动找你说话。",
      },
    ],
    ["meta", { property: "og:image", content: "/og.png" }],
    ["meta", { name: "twitter:card", content: "summary_large_image" }],
  ],
  locales: {
    root: {
      label: "简体中文",
      lang: "zh-CN",
      themeConfig: {
        nav: [
          { text: "首页", link: "/" },
          { text: "功能", link: "/features" },
          { text: "下载", link: "/download" },
          { text: "更新日志", link: "/changelog" },
          { text: "常见问题", link: "/faq" },
        ],
        sidebar: [
          { text: "功能介绍", link: "/features" },
          { text: "下载", link: "/download" },
          { text: "更新日志", link: "/changelog" },
          { text: "常见问题", link: "/faq" },
          { text: "隐私与关于", link: "/privacy" },
        ],
        footer: {
          message: "基于 Tauri 2 构建 · 用户数据全程本地 SQLCipher 加密",
          copyright: "Copyright © 2026 qiqi",
        },
        docFooter: { prev: "上一篇", next: "下一篇" },
        outline: { label: "本页目录" },
        lastUpdatedText: "最后更新",
        darkModeSwitchLabel: "主题",
        sidebarMenuLabel: "菜单",
        returnToTopLabel: "回到顶部",
        langMenuLabel: "语言",
      },
    },
    en: {
      label: "English",
      lang: "en-US",
      link: "/en/",
      themeConfig: {
        nav: [
          { text: "Home", link: "/en/" },
          { text: "Features", link: "/en/features" },
          { text: "Download", link: "/en/download" },
          { text: "Changelog", link: "/en/changelog" },
          { text: "FAQ", link: "/en/faq" },
        ],
        sidebar: [
          { text: "Features", link: "/en/features" },
          { text: "Download", link: "/en/download" },
          { text: "Changelog", link: "/en/changelog" },
          { text: "FAQ", link: "/en/faq" },
          { text: "Privacy", link: "/en/privacy" },
        ],
        footer: {
          message: "Built with Tauri 2 · Your data is encrypted locally with SQLCipher",
          copyright: "Copyright © 2026 qiqi",
        },
        docFooter: { prev: "Previous", next: "Next" },
        outline: { label: "On this page" },
        lastUpdatedText: "Last updated",
        sidebarMenuLabel: "Menu",
        returnToTopLabel: "Back to top",
        langMenuLabel: "Language",
      },
    },
  },
  themeConfig: {
    logo: "/logo.png",
    socialLinks: [{ icon: "github", link: repo }],
    search: { provider: "local" },
    externalLinkIcon: true,
  },
});
