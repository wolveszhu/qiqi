/**
 * 日记情绪徽标映射单测（F2.2 / D3）：`moodView` 统一入口。
 *
 * 契约以后端 `llm/diary.rs` 为真源：0=开心 / 1=平静 / 2=低落，null=未记录；
 * null/undefined/越界一律回退"平静"（D3 裁决 #11-2）。
 */

import { describe, expect, it } from "vitest";

import { MOOD_VIEW, moodView } from "../../constants/mood";
import type { DiaryMood } from "../../constants/mood";

describe("moodView", () => {
  it("0/1/2 映射到 emoji + 文案", () => {
    expect(moodView(0)).toEqual({ emoji: "😺", label: "开心" });
    expect(moodView(1)).toEqual({ emoji: "😌", label: "平静" });
    expect(moodView(2)).toEqual({ emoji: "😔", label: "低落" });
  });

  it("null / undefined → 平静兜底", () => {
    expect(moodView(null)).toEqual({ emoji: "😌", label: "平静" });
    expect(moodView(undefined)).toEqual({ emoji: "😌", label: "平静" });
  });

  it("越界值 → 平静兜底（后端未产出的防御）", () => {
    // 契约外值（后端升级/脏数据）：不得抛错，回落平静
    expect(moodView(7 as unknown as DiaryMood)).toEqual({ emoji: "😌", label: "平静" });
    expect(moodView(-1 as unknown as DiaryMood)).toEqual({ emoji: "😌", label: "平静" });
  });

  it("MOOD_VIEW 表恰好覆盖三个契约值", () => {
    expect(Object.keys(MOOD_VIEW)).toEqual(["0", "1", "2"]);
  });
});
