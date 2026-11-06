import { defineConfig } from "vitepress";

// 站点配置：中英双语，默认中文（root = zh），英文在 /en/。
// 所有自定义代码仅位于 website/，对应用零依赖。
const repo = "https://github.com/wolveszhu/qiqi";
const siteUrl = "https://qiqi.bootai.work";

// 根据页面相对路径推导 clean URL 形式的 canonical / hreflang 地址。
function canonicalOf(relativePath: string): string {
  let p = relativePath.replace(/\.md$/, "");
  if (p === "index") return siteUrl + "/";
  if (p.endsWith("/index")) p = p.slice(0, -"index".length); // 保留末尾斜杠
  return siteUrl + "/" + p;
}

// 百度统计（百度 Tongji）异步接入片段。
const baiduTongji = `
var _hmt = _hmt || [];
(function() {
  var hm = document.createElement("script");
  hm.src = "https://hm.baidu.com/hm.js?49f999c8105c4751d5df6157c8deb064";
  var s = document.getElementsByTagName("script")[0];
  s.parentNode.insertBefore(hm, s);
})();
`;

// 结构化数据：SoftwareApplication（提升搜索引擎对「桌面应用」的理解与富媒体展示）。
const appJsonLd = JSON.stringify({
  "@context": "https://schema.org",
  "@type": "SoftwareApplication",
  name: "七七 · 桌面数字生命",
  operatingSystem: "Windows",
  applicationCategory: "DesktopApplication",
  description:
    "七七是一款住在你桌面上的数字生命与 AI 桌宠：有性格、有记忆、有主动性，会主动找你说话，支持好感度进化、本地加密与双窗口桌宠。",
  url: siteUrl,
  offers: { "@type": "Offer", price: "0", priceCurrency: "CNY" },
  author: { "@type": "Organization", name: "qiqi" },
});

export default defineConfig({
  srcDir: "docs",
  title: "七七 · 桌面数字生命与 AI 桌宠",
  description:
    "七七是一款住在你桌面上的数字生命与 AI 桌宠：有性格、有记忆、有主动性，会主动找你说话，支持好感度进化、本地加密与双窗口桌宠。",
  lastUpdated: true,
  cleanUrls: true,
  appearance: false, // 果冻奶油风为浅色专属设计，关闭深浅切换
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
    // ---- SEO 基础 ----
    [
      "meta",
      {
        name: "keywords",
        content:
          "七七,桌宠,桌面宠物,数字生命,AI桌宠,虚拟宠物,桌面伙伴,电脑桌宠,Windows桌宠,二次元桌宠,本地加密,好感度,记忆,主动互动,果冻物理,Tauri,陪伴",
      },
    ],
    ["meta", { name: "application-name", content: "七七" }],
    ["meta", { name: "theme-color", content: "#FFF6E9" }],
    ["meta", { name: "robots", content: "index,follow,max-image-preview:large" }],
    // ---- Open Graph ----
    ["meta", { property: "og:type", content: "website" }],
    ["meta", { property: "og:site_name", content: "七七 · 桌面数字生命" }],
    ["meta", { property: "og:title", content: "七七 · 桌面数字生命与 AI 桌宠" }],
    [
      "meta",
      {
        property: "og:description",
        content:
          "住在你桌面上的数字生命与 AI 桌宠：有性格、有记忆、有主动性，支持好感度进化、本地加密与双窗口桌宠。",
      },
    ],
    ["meta", { property: "og:image", content: "/og.png" }],
    ["meta", { property: "og:locale", content: "zh_CN" }],
    ["meta", { property: "og:locale:alternate", content: "en_US" }],
    // ---- Twitter Card ----
    ["meta", { name: "twitter:card", content: "summary_large_image" }],
    ["meta", { name: "twitter:title", content: "七七 · 桌面数字生命与 AI 桌宠" }],
    [
      "meta",
      {
        name: "twitter:description",
        content:
          "住在你桌面上的数字生命与 AI 桌宠：有性格、有记忆、有主动性，支持好感度进化、本地加密与双窗口桌宠。",
      },
    ],
    ["meta", { name: "twitter:image", content: "/og.png" }],
    // ---- 结构化数据 ----
    ["script", { type: "application/ld+json" }, appJsonLd],
    // ---- 百度统计 ----
    ["script", {}, baiduTongji],
  ],
  // 按页面动态生成 canonical / og:url / hreflang 交替链接（双语 SEO 关键项）。
  transformHead: (ctx: any) => {
    const pd = ctx.pageData ?? ctx.page;
    const rel: string = pd?.filePathRelative || pd?.relativePath || ctx.page || "";
    const url = canonicalOf(rel);
    ctx.head.push(["link", { rel: "canonical", href: url }]);
    ctx.head.push(["meta", { property: "og:url", content: url }]);

    const altPath = rel.startsWith("en/")
      ? rel.replace(/^en\//, "")
      : rel === "index.md"
        ? "en/index.md"
        : "en/" + rel;
    const altUrl = canonicalOf(altPath);
    ctx.head.push(["link", { rel: "alternate", hreflang: "en", href: altUrl }]);
    ctx.head.push(["link", { rel: "alternate", hreflang: "zh-CN", href: url }]);
    return ctx.head;
  },
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
          { text: "隐私与关于", link: "/privacy" },
        ],
        sidebar: false,
        footer: {
          message: "基于 Tauri 2 构建 · 用户数据全程本地 SQLCipher 加密",
          copyright:
            'Copyright © 2026 qiqi<br><a href="https://beian.miit.gov.cn/" target="_blank" rel="noopener">浙ICP备2026060710号-1</a> · 友情链接：<a href="https://www.bootai.work/" target="_blank" rel="noopener">AI应用市场</a>',
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
          { text: "Privacy", link: "/en/privacy" },
        ],
        sidebar: false,
        footer: {
          message: "Built with Tauri 2 · Your data is encrypted locally with SQLCipher",
          copyright:
            'Copyright © 2026 qiqi<br><a href="https://beian.miit.gov.cn/" target="_blank" rel="noopener">浙ICP备2026060710号-1</a> · Friend link: <a href="https://www.bootai.work/" target="_blank" rel="noopener">AI应用市场</a>',
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
    search: false,
    externalLinkIcon: true,
  },
});
