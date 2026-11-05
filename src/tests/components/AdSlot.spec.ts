// @vitest-environment happy-dom
import { describe, it, expect } from "vitest";
import { mount } from "@vue/test-utils";

import AdBanner from "../../components/settings/AdBanner.vue";
import AdCard from "../../components/settings/AdCard.vue";
import { isDismissedToday, localToday } from "../../composables/useAds";
import type { AdItem } from "../../config/ads";

const banner: AdItem = {
  id: "banner-test",
  slot: "top-banner",
  text: "七七的新形象正在准备中",
  tone: "matcha",
  link: "https://example.com",
};
const card: AdItem = {
  id: "card-test",
  slot: "sidebar-card",
  image: "sidebar-300x250.png",
  link: "https://example.com",
};

describe("广告关闭的本地今日比较", () => {
  it("按本地时区格式化 YYYY-MM-DD（23:30 不因 UTC 偏移跳日）", () => {
    expect(localToday(new Date(2026, 10, 4, 23, 30))).toBe("2026-11-04");
  });

  it("今日已关 / 昨日恢复 / 未关闭", () => {
    expect(isDismissedToday("2026-11-04", "2026-11-04")).toBe(true);
    expect(isDismissedToday("2026-11-03", "2026-11-04")).toBe(false);
    expect(isDismissedToday(undefined, "2026-11-04")).toBe(false);
  });
});

describe("AdBanner 顶部文字横幅", () => {
  it("渲染「广告」标识与文案（无图片）", () => {
    const w = mount(AdBanner, { props: { ad: banner } });
    expect(w.find(".ad-banner__tag").text()).toBe("广告");
    expect(w.text()).toContain("七七的新形象正在准备中");
    expect(w.find("img").exists()).toBe(false);
  });

  it("点「了解详情」emit open、点关闭 emit dismiss", async () => {
    const w = mount(AdBanner, { props: { ad: banner } });
    await w.find(".ad-banner__link").trigger("click");
    await w.find(".ad-banner__close").trigger("click");
    expect(w.emitted("open")).toHaveLength(1);
    expect(w.emitted("dismiss")).toHaveLength(1);
  });

  it("整块横幅（标签/文案/空白）点击均 emit open", async () => {
    const w = mount(AdBanner, { props: { ad: banner } });
    await w.find(".ad-banner__tag").trigger("click");
    await w.find(".ad-banner__text").trigger("click");
    await w.find(".ad-banner").trigger("click");
    expect(w.emitted("open")).toHaveLength(3);
    expect(w.emitted("dismiss")).toBeUndefined();
  });

  it("点关闭只 emit dismiss，不会顺带触发跳转", async () => {
    const w = mount(AdBanner, { props: { ad: banner } });
    await w.find(".ad-banner__close").trigger("click");
    expect(w.emitted("dismiss")).toHaveLength(1);
    expect(w.emitted("open")).toBeUndefined();
  });
});

describe("AdCard 侧边栏图片卡", () => {
  it("6:5 素材按 144×120 显示，带广告角标", () => {
    const w = mount(AdCard, { props: { ad: card } });
    const img = w.find(".ad-card__img");
    expect(img.attributes("width")).toBe("144");
    expect(img.attributes("height")).toBe("120");
    expect(img.attributes("src")).toContain("sidebar-300x250");
    expect(w.find(".ad-card__tag").text()).toBe("广告");
  });

  it("点卡片 emit open；关闭按钮只 emit dismiss（不冒泡触发 open）", async () => {
    const w = mount(AdCard, { props: { ad: card } });
    await w.find(".ad-card__hit").trigger("click");
    await w.find(".ad-card__close").trigger("click");
    expect(w.emitted("open")).toHaveLength(1);
    expect(w.emitted("dismiss")).toHaveLength(1);
  });
});
