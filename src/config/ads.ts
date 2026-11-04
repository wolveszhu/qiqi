/**
 * 广告位内容（本地内置单一真源；随版本更新，无网络依赖）。
 *
 * 纪律：`id` 是关闭持久化的键——**换素材必须换 id**，否则沿用旧 id 会被
 * 用户上次「今日不再显示」继续抑制。图片素材规格见 `src/assets/ad/`。
 */

import sidebarPlaceholder from "../assets/ad/sidebar-300x250.png";

/** 广告位定义（文案/图片/链接均为本地常量）。 */
export interface AdItem {
  /** 素材唯一 ID（关闭持久化的键；换素材必换 ID）。 */
  id: string;
  /** 展示位置（与组件一一对应）。 */
  slot: "top-banner" | "sidebar-card";
  /** 文字横幅文案（top-banner）。 */
  text?: string;
  /** 横幅背景色 token（top-banner）。 */
  tone?: "matcha" | "peach";
  /** 图片 URL（sidebar-card；素材规格 300×250）。 */
  image?: string;
  /** 点击跳转链接（经系统浏览器打开）。 */
  link: string;
}

/** 顶部文字横幅：不用图片，文字 + 背景色 + 跳转链接。 */
export const TOP_BANNER_AD: AdItem = {
  id: "top-banner-2026-11",
  slot: "top-banner",
  text: "专属通道！阿里云新客首购8.5折，叠加15%补贴，手慢无！",
  tone: "matcha",
  link: "https://www.aliyun.com/minisite/goods?userCode=50kjssue",
};

/** 侧边栏图片卡：素材 300×250（IAB medium rectangle），显示时等比缩放为 144×120。 */
export const SIDEBAR_CARD_AD: AdItem = {
  id: "sidebar-2026-11",
  slot: "sidebar-card",
  image: sidebarPlaceholder,
  link: "https://www.aliyun.com/benefit/client/cross?userCode=50kjssue",
};

/** 全部广告位（组件按 slot 取用；新增广告位在此追加）。 */
export const AD_ITEMS: readonly AdItem[] = [TOP_BANNER_AD, SIDEBAR_CARD_AD];
