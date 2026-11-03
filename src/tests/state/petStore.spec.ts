/**
 * petStore 单测（AV5：形象 id 的持久化读出）。
 *
 * 覆盖 `applyPetState` 是否把后端下发的 `activeRenderer` 写进镜像——
 * 此前该字段从未同步，导致 `activeRenderer` 恒为初始值 `code_cat`，
 * 形象选择无论怎么切都读不回来（重启亦然）。
 */

import { beforeEach, describe, expect, it } from "vitest";

import type { PetStateDto } from "../../api/types";
import { applyPetState, petStore } from "../../state/petStore";

function dto(activeRenderer: string): PetStateDto {
  return {
    petName: "喵喵",
    relationshipLevel: 1,
    affectionPoints: 0,
    lastInteraction: null,
    activeRenderer,
  };
}

describe("applyPetState", () => {
  beforeEach(() => {
    applyPetState(dto("code_cat"));
  });

  it("同步好感度与名字字段", () => {
    applyPetState({
      petName: "团子",
      relationshipLevel: 2,
      affectionPoints: 320,
      lastInteraction: "2026-10-26",
      activeRenderer: "code_cat",
    });
    expect(petStore.petName).toBe("团子");
    expect(petStore.relationshipLevel).toBe(2);
    expect(petStore.affectionPoints).toBe(320);
    expect(petStore.lastInteraction).toBe("2026-10-26");
  });

  it("写入 activeRenderer（形象选择能被读回）", () => {
    applyPetState(dto("code_dog"));
    expect(petStore.activeRenderer).toBe("code_dog");
  });

  it("空值不覆盖既有形象（防御后端缺省）", () => {
    applyPetState(dto("code_dog"));
    applyPetState(dto(""));
    expect(petStore.activeRenderer).toBe("code_dog");
  });
});
