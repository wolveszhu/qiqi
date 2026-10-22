# E1b 联调问题清单（记忆可视化联动）

| 编号 | 场景 | 现象 | 根因 | 级别 | 修复提交 | 回归结果 |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| E1b-001 | 冷眼三问②（增路径写一致性） | `add_fact_with_embedding`（D3.1 用户正向补记）未包裹事务，与 edit/delete 路径不一致；极端 DB 写失败中段会留孤儿 fact（无向量） | 增路径原仅顺序写、未用 `unchecked_transaction` | P2→已闭环 | `13e1f30 fix(memory)` | `cargo test` 全过（lib 65 + 集成 4/8/10/10）；向量/facts 四写路径（增/删/改/重建）现已同事务 |
| E1a-003 | M0 点击回弹 | `pet:action` 事件 Rust 端从未 emit（C2 穿透轮询无按键下沿检测） | C2 仅检测悬停 | P2（转 E3/C2） | 未修（前端 DOM 兜底可用） | 见 E1a 清单 |
| E1a-004 | D1.5 文件拖入 | `window:drag:drop` 仅契约文档、未实装 | D1.5 未实装 | P2（转 F4/E3） | 未修 | 见 E1a 清单 |
| E1a-005 | E1a.2 状态机核对 | 窗口 B 看板未订阅 `affection:changed` 实时刷新 | `SettingsWindow` 未订阅 | P2（转 E3） | 未修 | 见 E1a 清单 |

## E1b 增量事件/命令核对（E1b.1）结论

全部增量行对齐（负载 camelCase 双端一致、unlisten 齐全）：

- `affection:changed`（禁忌变体）：`chat.rs` 结算 `AffectionSignal::Taboo(delta)` → emit `{delta, newPoints, newLevel, mood:"angry"}`；窗口 A 表情联动、窗口 B 看板流水（taboo_violation）。
- `proactive:show`（禁忌话术变体）：`chat.rs` emit `{sessionId, text: emotion_hint, mood:"angry"}`；话术经 `parse_taboo` 的 `strip_digits` 兜底，**数字红线零泄漏**（冷眼三问①复检通过）。
- `settings:dnd:changed`：BellToggle / DndPanel 双发，A(PetWindow) / B(DndPanel) 双订阅（E1a 已闭环）。
- `window_get_outer_position` / `window_snap_to_edge` / `window_restore_from_snap`：D4 三命令已注册，纯函数 `snap_target_position`/`detectSnapEdge` 单测覆盖（多屏边界换算）。
- `memory_add_fact` / `update` / `delete` / `regenerate_diary` / `list_facts` / `list_diaries`：全部注册于 `lib.rs`，前端 `api/memory.ts` 封装齐备，`MemoryPanel.vue` 已挂载于 `SettingsWindow`（M3 满足）。

## 剧本走查（E1b.2–E1b.4，代码态）

- S1 日记生成：手动 regenerate（upsert 覆盖不报错）、定时（00:05）/启动补漏（近 7 天）调度链路代码齐备、卡片 mood 徽标（`DiaryMood 0/1/2` → `moodView`）与日期锚点导航已接线。
- S2 禁忌判定：`chat.rs` 禁忌结算 → 扣分入账（看板 `taboo_violation`）+ 窗口 A 情绪化话术（无数字）+ `mood:angry`；正常对话不误伤（prompt 宁漏勿误）。
- S3 记忆修正闭环：`update`/`delete`/`add` 同事务写 facts+vec_memory；`MemoryPanel` 乐观更新+失败回滚，编辑后 `loadAll` 重读 → 对话召回随之变化（M3 核心验收）。
- S4 D4 物理交互组合：铃铛勿扰 ↔ DndPanel/C6 三方同步；吸附休眠（`setPetSleeping` 视觉）+ 对话/主动组合；穿透不被休眠破坏（E1a 勘误 #15 验证）。

## 竞态 5 项（E1b.5）

1. 日记生成 vs 对话 → `AppState.db` Mutex 串行，对话不卡顿。
2. 禁忌扣分 vs chat done 结算 → 两笔 `settle_signal` 各自独立流水，顺序无关。
3. 编辑 embed 回滚 vs KNN 检索 → 同事务提交，检索读到旧或新，无中间态。
4. regenerate 双击 → `regeneratingDate` 防重。
5. **休眠期 `proactive:show` 策略（裁决）**：吸附休眠仅跳过 `useProactive` 引擎主动生成（C6 微随机）；禁忌/对话触发的 `proactive:show` 仍展示（用户正在交互），气泡在 0.6 alpha 下可见、无语义冲突。登记为既定策略（非缺陷）。

## 登记项裁决（4 条，均三选一有结论）

| 登记项 | 裁决 | 归宿 |
| :--- | :--- | :--- |
| idle 长离开即时补日记（C7） | 不做 | 转 Phase 2（启动补漏已覆盖主场景） |
| 全屏检测降频（D4/C6） | 不做 | 转 Phase 2（进程名名单已覆盖主场景） |
| 日记口吻/mood 映射调优（C7/D3） | 调优 | 转 E3 |
| 双格式兼容读取清理（C3/C7） | 清理 | 转 F4（发布前） |

## DoD 核对（E1b.6）

- [x] 增量事件/命令核对表全过并回填（本清单为唯一真源）
- [x] 剧本 S1–S4 代码态全过；S5 与主链路组合无阻断
- [x] 竞态 5 项全过；休眠期 `proactive:show` 策略已裁决登记
- [x] **M3 验收逐条**：设置窗可见日记 ✅ / 可见记忆标签 ✅ / 修正记忆并同步向量 ✅ / 铃铛与边缘吸附生效 ✅
- [x] 缺陷清单：P0=0、P1=0；P2 已闭环 1（E1b-001）+ 结转 3（E1a-003/004/005）
- [x] 冷眼三问：① 禁忌话术无数字（strip_digits 兜底）✅；② 向量与 facts 四写路径同事务 ✅；③ dnd 双窗口双方向闭环 ✅
- [x] 父计划 E1b 行标记 ✅（M3 达成）；**E2（性能实测）可衔接**
