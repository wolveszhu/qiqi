/**
 * 国际化基座契约（中英文切换需求）：
 * - OS 语言推断（中文系→zh-CN，其它→en-US）；
 * - 两份字典键集合一致（编译期已由 `satisfies` 约束，此处再校验运行时值）；
 * - 切换 `i18n.global.locale` 后同一 key 返回对应译文。
 */

import { describe, expect, it } from "vitest";

import i18n, { DEFAULT_LOCALE, SUPPORTED_LOCALES, detectLocale } from "../../i18n";
import enUS from "../../i18n/locales/en-US";
import zhCN from "../../i18n/locales/zh-CN";

/** 递归收集字典的叶子键路径（如 "common.save"）。 */
function leafKeys(obj: Record<string, unknown>, prefix = ""): string[] {
  const out: string[] = [];
  for (const [key, value] of Object.entries(obj)) {
    const path = prefix ? `${prefix}.${key}` : key;
    if (typeof value === "object" && value !== null) {
      out.push(...leafKeys(value as Record<string, unknown>, path));
    } else {
      out.push(path);
    }
  }
  return out.sort();
}

/** 递归收集字典的叶子值（支持多层分区，如 `translate.popup.copy`）。 */
function leafValues(obj: Record<string, unknown>): unknown[] {
  const out: unknown[] = [];
  for (const value of Object.values(obj)) {
    if (typeof value === "object" && value !== null) {
      out.push(...leafValues(value as Record<string, unknown>));
    } else {
      out.push(value);
    }
  }
  return out;
}

describe("detectLocale", () => {
  it("中文系 OS 语言 → zh-CN", () => {
    expect(detectLocale("zh-CN")).toBe("zh-CN");
    expect(detectLocale("zh-TW")).toBe("zh-CN");
    expect(detectLocale("ZH-Hans")).toBe("zh-CN");
  });

  it("其它 OS 语言 → en-US", () => {
    expect(detectLocale("en-US")).toBe("en-US");
    expect(detectLocale("ja-JP")).toBe("en-US");
    expect(detectLocale("")).toBe("en-US");
  });
});

describe("locale constants", () => {
  it("白名单与默认语言", () => {
    expect(SUPPORTED_LOCALES).toEqual(["zh-CN", "en-US"]);
    expect(DEFAULT_LOCALE).toBe("zh-CN");
  });
});

describe("dictionaries", () => {
  it("zh-CN 与 en-US 键集合完全一致", () => {
    const zh = leafKeys(zhCN as unknown as Record<string, unknown>);
    const en = leafKeys(enUS as unknown as Record<string, unknown>);
    expect(en).toEqual(zh);
  });

  it("所有叶子值均为非空字符串", () => {
    for (const dict of [zhCN, enUS]) {
      for (const leaf of leafValues(dict as unknown as Record<string, unknown>)) {
        expect(typeof leaf).toBe("string");
        expect((leaf as string).length).toBeGreaterThan(0);
      }
    }
  });
});

describe("i18n instance", () => {
  it("切换 locale 后同一 key 返回对应译文", () => {
    i18n.global.locale.value = "zh-CN";
    expect(i18n.global.t("sideNav.general")).toBe("通用");
    expect(i18n.global.t("general.title")).toBe("通用");

    i18n.global.locale.value = "en-US";
    expect(i18n.global.t("sideNav.general")).toBe("General");
    expect(i18n.global.t("general.title")).toBe("General");

    // 还原，避免影响同进程内其它用例
    i18n.global.locale.value = DEFAULT_LOCALE;
  });

  it("插值参数生效", () => {
    expect(i18n.global.t("chat.messageCount", { n: 3 })).toBe("3 条");
    i18n.global.locale.value = "en-US";
    expect(i18n.global.t("chat.messageCount", { n: 3 })).toBe("3 messages");
    i18n.global.locale.value = DEFAULT_LOCALE;
  });
});
