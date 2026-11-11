//! 鼠标坐标探针：**全应用唯一的 `GetCursorPos` 封装**（规范 §4 硬约束：禁止另开轮询）。
//!
//! 消费方：`probe/scheduler` 每 100ms 轮询写缓存并 emit `probe:cursor:move`（视线跟随）；
//! 穿透同步（click_through）每轮**现取**一次——缓存来自另一条 100ms 线程，快速移动后
//! 点击时最坏滞后 200ms 才解除穿透，现取把延迟砍半。两者共用本封装，不是双轮询。

use windows::Win32::Foundation::POINT;
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

use crate::error::AppError;

/// 读取当前鼠标**屏幕坐标**；Win32 调用失败映射 `AppError::Probe`。
pub fn current_position() -> Result<(i32, i32), AppError> {
    let mut point = POINT::default();
    unsafe { GetCursorPos(&mut point) }
        .map_err(|e| AppError::Probe(format!("GetCursorPos 失败: {e}")))?;
    Ok((point.x, point.y))
}
