//! 全局状态容器：命令层经 `tauri::State` 访问（规范 §4）。
//!
//! - `db`：SQLCipher 连接（B4 注入，setup 内 `app.manage`）；
//! - `hit_region`/`cursor`/`last_ignore`：C2 穿透状态机的进程内缓存（**不走事件**，零 IPC 洪泛）；
//! - `click_through_enabled`：穿透总闸（D1 输入框可见 / D4 边缘休眠时关闭）。

use std::sync::Mutex;

use rusqlite::Connection;

/// 命中区域（**窗口局部坐标**；前端按 `getHitRegion()` 上报，变化时 invoke）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HitRegion {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl HitRegion {
    /// 区域是否有效（未上报/尺寸为 0 视为无效 → 穿透回退默认开启）。
    pub fn is_valid(&self) -> bool {
        self.width > 0 && self.height > 0
    }
}

/// 应用级状态：经 `app.manage(AppState)` 注册，命令层经 `State<AppState>` 访问。
pub struct AppState {
    /// SQLCipher 连接（B4 注入；C6 穿透状态机的 cursor/hit_region 见下）。
    pub db: Mutex<Connection>,
    /// 命中区域缓存（C2.2：前端低频上报写入）。
    pub hit_region: Mutex<HitRegion>,
    /// 鼠标屏幕坐标缓存（C2.3：探针单点轮询写入）。
    pub cursor: Mutex<(i32, i32)>,
    /// 上一次应用到窗口的穿透状态（C2.4：仅翻转时调用系统 API）。
    pub last_ignore: Mutex<Option<bool>>,
    /// 穿透总闸：true=启用自动穿透（默认）；false=整窗接收点击（D1/D4 场景）。
    pub click_through_enabled: Mutex<bool>,
    /// 空闲状态机当前值（C6：true=idle；scheduler 翻转时 emit，省事件洪泛）。
    pub idle_active: Mutex<bool>,
    /// 前台进程名缓存（C6：变化时才 emit `probe:foreground:changed`）。
    pub last_foreground: Mutex<Option<String>>,
}

impl AppState {
    /// 构造初始状态（db 由调用方注入）。
    pub fn new(db: Mutex<Connection>) -> Self {
        Self {
            db,
            hit_region: Mutex::new(HitRegion::default()),
            cursor: Mutex::new((0, 0)),
            last_ignore: Mutex::new(None),
            click_through_enabled: Mutex::new(true),
            idle_active: Mutex::new(false),
            last_foreground: Mutex::new(None),
        }
    }
}
