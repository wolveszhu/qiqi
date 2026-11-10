-- M15：自定义导入形象表（Phase 3 自定义形象）。
--
-- 单用户可导入多个自定义形象，与内置猫狗在设置面板并列管理。
-- 形象图片本体不入库（库已全库 SQLCipher 加密，但二进制大对象仍走文件系统更合适），
-- 仅存**相对**路径 `rel_path`（相对 app_data_dir），图片由 Rust 命令拷入
-- app_data_dir/custom_avatars/<id>/avatar.png。
--
-- 缩放/锚点为「整体变换」渲染所需的几何参数（自定义精灵图无矢量分层，无眨眼/视线）：
-- - scale：整体缩放（默认 1）；
-- - anchor_x / anchor_y：圆盘中心相对图片宽高的归一化坐标（0..1，默认 0.5），
--   即边缘吸附躺倒时旋转半圆的圆心基准，保证躺倒露出的是用户期望的部位。
--
-- 幂等：CREATE TABLE IF NOT EXISTS（无数据种子，用户自导）。

CREATE TABLE IF NOT EXISTS custom_avatars (
    id          TEXT    PRIMARY KEY,
    name        TEXT    NOT NULL,
    rel_path    TEXT    NOT NULL,
    scale       REAL    NOT NULL DEFAULT 1.0,
    anchor_x    REAL    NOT NULL DEFAULT 0.5,
    anchor_y    REAL    NOT NULL DEFAULT 0.5,
    created_at  TEXT    NOT NULL,
    updated_at  TEXT    NOT NULL
);
