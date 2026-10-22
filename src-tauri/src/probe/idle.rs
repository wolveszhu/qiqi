//! 空闲探针：**只读系统输入状态**（规范 §4.1 硬约束，禁止写入）。
//!
//! `GetLastInputInfo` 返回上次用户输入的 `tickCount`（与 `GetTickCount` 同源），
//! 与当前 `GetTickCount` 之差即空闲时长；`wrapping_sub` 自然处理 49.7 天回绕。

use windows::Win32::System::SystemInformation::GetTickCount;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

use crate::error::AppError;

/// 当前空闲时长（毫秒）；Win32 调用失败映射 `AppError::Probe`。
pub fn idle_ms() -> Result<u64, AppError> {
    let mut info = LASTINPUTINFO { cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32, dwTime: 0 };
    // 返回 BOOL（非 Result）：0 = 失败
    let ok = unsafe { GetLastInputInfo(&mut info) };
    if !ok.as_bool() {
        return Err(AppError::Probe("GetLastInputInfo 失败".into()));
    }
    let now = unsafe { GetTickCount() };
    // dwTime 与 GetTickCount 同为 DWORD（u32），wrapping_sub 处理回绕得正确空闲差
    Ok(now.wrapping_sub(info.dwTime) as u64)
}
