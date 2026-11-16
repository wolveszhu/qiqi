//! UIA 只读选区探测（2026-11-16 零污染改造）。
//!
//! 与旧的剪贴板法根本区别：**全程不写剪贴板、不清空剪贴板、不注入 `Ctrl+C`**。
//! 手势结束时从焦点元素的 `TextPattern` 只读取得选区文本并交给调用方存进内存，
//! 用户点「翻译」时直接用内存文本，不再依赖焦点或剪贴板。
//!
//! UIA COM 要求 STA，故所有 COM 调用集中在本模块的专用线程；调用方通过
//! `mpsc` 提交请求并等待有界超时，绝不阻塞 100ms 探针轮询超过预算。

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::OnceLock;
use std::time::Duration;

use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Accessibility::{
    IUIAutomation, IUIAutomationTextPattern, CUIAutomation, UIA_TextPatternId,
};

use crate::error::AppError;

/// 单次只读探测的请求：回复 `Some(选区文本)` 或 `None`（无选区/不可读）。
struct ProbeRequest {
    reply: Sender<Option<String>>,
}

static UIA_SENDER: OnceLock<Sender<ProbeRequest>> = OnceLock::new();

/// UIA 探测硬超时：必须显著小于探针轮询间隔，避免冻结 cursor 缓存。
pub const UIA_PROBE_TIMEOUT_MS: u64 = 80;

/// 启动 UIA STA 线程（进程内单例）。重复调用无副作用。
pub fn start_uia_probe() {
    let (tx, rx) = mpsc::channel::<ProbeRequest>();
    if UIA_SENDER.set(tx).is_err() {
        return;
    }
    std::thread::spawn(move || run_sta(rx));
}

/// 只读探测焦点元素的选区文本；失败/无选区返回 `None`，不触发任何降级取词。
pub fn probe_selection() -> Option<String> {
    let tx = UIA_SENDER.get()?;
    let (reply_tx, reply_rx) = mpsc::channel();
    tx.send(ProbeRequest { reply: reply_tx }).ok()?;
    reply_rx.recv_timeout(Duration::from_millis(UIA_PROBE_TIMEOUT_MS)).ok().flatten()
}

/// 供前端错误提示使用：当前选区失效或 UIA 不可达时的可行动文案。
pub fn selection_unavailable_error() -> AppError {
    AppError::Probe("当前窗口无法读取选区，请手动复制后使用".into())
}

fn run_sta(rx: Receiver<ProbeRequest>) {
    let init = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    if init.is_err() {
        eprintln!("[probe] UIA 线程 COM 初始化失败，划词工具条不可用");
        return;
    }
    let created: windows::core::Result<IUIAutomation> =
        unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL) };
    let Ok(automation) = created else {
        eprintln!("[probe] UIA 自动化对象创建失败，划词工具条不可用");
        unsafe { CoUninitialize() };
        return;
    };

    loop {
        match rx.recv_timeout(Duration::from_secs(1)) {
            Ok(req) => {
                let _ = req.reply.send(read_selection(&automation));
            },
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    unsafe { CoUninitialize() };
}

/// 在 STA 线程中只读取得焦点元素的选区文本（概念链见计划文档 §4.2）。
fn read_selection(automation: &IUIAutomation) -> Option<String> {
    unsafe {
        let focused = automation.GetFocusedElement().ok()?;
        let pattern: IUIAutomationTextPattern =
            focused.GetCurrentPatternAs(UIA_TextPatternId).ok()?;
        let ranges = pattern.GetSelection().ok()?;
        let len = ranges.Length().ok()?;
        if len <= 0 {
            return None;
        }
        let mut out = String::new();
        for index in 0..len {
            let range = ranges.GetElement(index).ok()?;
            let text = range.GetText(-1).ok()?;
            out.push_str(&text.to_string());
        }
        (!out.trim().is_empty()).then_some(out)
    }
}
