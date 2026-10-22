//! 前台进程探针：**仅取进程名**（规范 §4.1 隐私红线——绝不读窗口标题/内容）。
//!
//! 链路：`GetForegroundWindow` → `GetWindowThreadProcessId` → `OpenProcess`
//! → `QueryFullProcessImageNameW` → 取可执行文件名。任何一步失败返回 `None`
//! （视为非工作场景，正常概率，不报错不崩溃——风险对策 §C6）。

use std::path::Path;

use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

/// `OpenProcess` 句柄 RAII 守卫：作用域结束自动 `CloseHandle`（防句柄泄漏）。
struct ProcessGuard(windows::Win32::Foundation::HANDLE);

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

/// 当前前台进程名（小写可执行文件名，如 `code.exe`）；无前台/失败返回 `None`。
pub fn foreground_process_name() -> Option<String> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return None;
        }
        let handle = match OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) {
            Ok(h) => h,
            Err(_) => return None,
        };
        let _guard = ProcessGuard(handle);
        let mut buf = [0u16; 1024];
        let mut size: u32 = buf.len() as u32;
        // Win32 格式 + PWSTR 缓冲（windows 0.58 签名）
        match QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_FORMAT(0),
            windows::core::PWSTR(buf.as_mut_ptr()),
            &mut size,
        ) {
            Ok(()) => {
                let len = (size as usize).min(buf.len());
                let path = String::from_utf16_lossy(buf.get(..len).unwrap_or(&[]));
                Path::new(&path).file_name().and_then(|s| s.to_str()).map(|s| s.to_string())
            },
            Err(_) => None,
        }
    }
}
