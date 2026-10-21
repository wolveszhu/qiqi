//! 鼠标坐标探针：**全应用唯一的 `GetCursorPos` 调用点**（规范 §4 硬约束）。
//!
//! 单点原则：穿透判断（click_through）与视线跟随（lookAt）共用本文件的轮询结果——
//! 穿透读进程内缓存、lookAt 走 `probe:cursor:move` 事件，杜绝双轮询。

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
