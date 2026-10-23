//! SQLCipher 加密连接初始化（规范 §4.1 / rule.md §H）：
//! 密钥（Windows 凭据管理器）→ sqlite-vec auto-extension → 打开连接 → `PRAGMA key`
//! → 版本三分支（等于跳过 / 小于备份后迁移 / 大于失败关闭）。
//!
//! 边界：本文件是数据层唯一"组装"入口；业务 SQL 一律进 repositories/（B5）。
//!
//! rusqlite_migration 1.3.1 API 实态（实施时查证）：
//! - `current_version(&Connection) -> SchemaVersion`（经 `usize::from` 取数值）；
//! - `to_latest(&mut Connection)`——需可变借用，故 `apply_migrations` 收 `&mut`。

use std::{path::Path, sync::Mutex};

use rusqlite::Connection;

use super::migrations;
use crate::error::AppError;

/// 管理态容器：`.setup()` 中 `app.manage(...)`，命令层经 `State<AppState>` 访问。
#[derive(Debug)]
pub struct Db {
    pub conn: Mutex<Connection>,
}

/// 初始化入口：密钥 → vec 注册 → 打开加密连接 → 迁移。
pub fn init(db_path: &Path) -> Result<Db, AppError> {
    let key = load_or_create_key()?;
    let mut conn = open_encrypted(db_path, &key)?;
    apply_migrations(&mut conn, db_path)?;
    Ok(Db { conn: Mutex::new(conn) })
}

/// 打开加密连接（vec 注册 → open → `PRAGMA key` → 可解密性校验）。
///
/// 故障防御（B4 冷启动实测）：不可解密时**先关闭连接再隔离**——
/// Windows 下文件句柄未释放时 `rename` 会报 os error 32（用例 8 实测坑）。
fn open_encrypted(db_path: &Path, key: &str) -> Result<Connection, AppError> {
    register_vec_extension()?;

    if let Some(dir) = db_path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| AppError::Io(e.to_string()))?;
    }
    let conn = Connection::open(db_path).map_err(AppError::from)?;
    // PRAGMA key 必须是连接上第一条语句；密钥为本地生成 hex（非用户输入），
    // 不适用参数化查询约束（rule.md §H 的参数化针对业务数据 SQL）。
    conn.execute_batch(&format!("PRAGMA key = \"x'{key}'\";")).map_err(AppError::from)?;

    match ensure_decryptable(&conn) {
        Ok(()) => Ok(conn),
        Err(AppError::Config(msg)) => {
            // 隔离前必须释放句柄（Windows 文件占用）
            drop(conn);
            let corrupt = quarantine_db(db_path)?;
            Err(AppError::Config(format!("{msg} 已隔离为 {}，重启后将重建。", corrupt.display())))
        },
        Err(e) => {
            drop(conn);
            Err(e)
        },
    }
}

/// 可解密性判定：读取 `PRAGMA user_version` 试探（不产生副作用）。
///
/// 起因（B4 冷启动真实故障）：keyring 未启用平台后端时"看似成功"但不持久化
/// → 每次启动生成新随机密钥 → 旧库以新密钥打开报
/// `NotADatabase(26) file is not a database`。
fn ensure_decryptable(conn: &Connection) -> Result<(), AppError> {
    match conn.query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0)) {
        Ok(_) => Ok(()),
        Err(rusqlite::Error::SqliteFailure(e, _))
            if e.code == rusqlite::ffi::ErrorCode::NotADatabase =>
        {
            Err(AppError::Config("数据库无法用当前密钥打开（文件损坏或密钥已丢失）。".into()))
        },
        Err(e) => Err(AppError::from(e)),
    }
}

/// 隔离打不开的库：重命名为 `pet.db.corrupt-<unix秒>`——**保留证据，绝不删除**。
/// 调用前必须关闭该文件的连接句柄。
fn quarantine_db(db_path: &Path) -> Result<std::path::PathBuf, AppError> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default();
    let corrupt = db_path.with_file_name(format!("pet.db.corrupt-{stamp}"));
    std::fs::rename(db_path, &corrupt).map_err(|e| AppError::Io(e.to_string()))?;
    Ok(corrupt)
}

// 历史实现注释（已被 ensure_decryptable + quarantine_db 取代，保留原因说明）：
// 初版在连接未关闭时直接 rename，Windows 报 os error 32（文件被占用），
// 故改为"判定与隔离分离、隔离前 drop 连接"——见 open_encrypted 与用例 8。

/// 版本三分支（附录 A §A.6）：版本读取以 rusqlite_migration 为准，禁止手工覆写。
///
/// 备份语义（B4.7 实测修正）：仅**前向迁移已有数据的库**（current > 0）才备份——
/// 空库（current == 0）无可保数据；注意 `Connection::open` 会创建空文件，
/// 不能以 `exists()` 判断是否需要备份（B4 实施时的真实坑）。
fn apply_migrations(conn: &mut Connection, db_path: &Path) -> Result<(), AppError> {
    let chain = migrations::chain();
    let current: usize = chain.current_version(conn).map_err(AppError::from)?.into();
    let target = migrations::TARGET_VERSION;

    match current.cmp(&target) {
        std::cmp::Ordering::Equal => Ok(()),
        std::cmp::Ordering::Greater => Err(AppError::Config(format!(
            "数据库版本 {current} 高于应用支持的 {target}，请升级应用"
        ))),
        // 空库：直接迁移（无备份——无数据可保）
        std::cmp::Ordering::Less if current == 0 => chain.to_latest(conn).map_err(AppError::from),
        // 前向迁移：先备份（rule.md §5）与库同目录的 pet.db.bak
        std::cmp::Ordering::Less => {
            let backup = db_path.with_extension("db.bak");
            std::fs::copy(db_path, &backup).map_err(|e| AppError::Io(e.to_string()))?;
            chain.to_latest(conn).map_err(AppError::from)
        },
    }
}

/// Windows 凭据管理器取/建 32 字节 hex 密钥。
///
/// CI/无凭据环境旁路：测试代码走 `init_with_key`（见下），生产路径只走 keyring。
fn load_or_create_key() -> Result<String, AppError> {
    use keyring::Entry;
    let entry = Entry::new(crate::config::KEY_SERVICE, crate::config::KEY_USER)
        .map_err(|e| AppError::Io(e.to_string()))?;
    match entry.get_password() {
        Ok(key) => Ok(key),
        Err(keyring::Error::NoEntry) => {
            let mut buf = [0u8; 32];
            getrandom::getrandom(&mut buf).map_err(|e| AppError::Io(e.to_string()))?;
            let key: String = buf.iter().map(|b| format!("{b:02X}")).collect();
            entry.set_password(&key).map_err(|e| AppError::Io(e.to_string()))?;
            Ok(key)
        },
        Err(e) => Err(AppError::Io(e.to_string())),
    }
}

/// sqlite-vec 自动扩展注册。
///
/// ⚠️ 时序约束：必须在 `Connection::open` **之前**调用——auto_extension 列表
/// 在新连接打开时逐个尝试加载 vec0 模块；顺序错 → M2 报 `no such module: vec0`。
/// 进程内重复调用幂等（SQLite 去重），测试可每用例独立调用。
/// SQLite 扩展入口函数类型（`sqlite3_auto_extension` 期望的签名）。
///
/// sqlite-vec 0.1.9 的 `sqlite3_vec_init` 声明为无参 C 原型（`pub fn ...;`），
/// 无法直接作为入口传入——先转 `*const ()` 再 transmute 为带签名的函数指针
/// （fn item 零尺寸不可直接 transmute；turbofish 显式注解满足 clippy）。
type VecExtensionInit = unsafe extern "C" fn(
    *mut rusqlite::ffi::sqlite3,
    *mut *mut std::ffi::c_char,
    *const rusqlite::ffi::sqlite3_api_routines,
) -> std::ffi::c_int;

pub fn register_vec_extension() -> Result<(), AppError> {
    use rusqlite::ffi;
    unsafe {
        let init = std::mem::transmute::<*const (), VecExtensionInit>(
            sqlite_vec::sqlite3_vec_init as *const (),
        );
        let rc = ffi::sqlite3_auto_extension(Some(init));
        if rc != ffi::SQLITE_OK {
            return Err(AppError::Db(format!("sqlite-vec auto extension 注册失败 rc={rc}")));
        }
        // 触发 SQLite 库初始化，确保后续 open 时扩展生效
        ffi::sqlite3_initialize();
    }
    Ok(())
}

/// 测试旁路：以固定密钥直接打开加密连接（绕开 keyring，仅供集成测试）。
/// 公开而非 `pub(crate)`：`tests/migrations_it.rs` 集成测试需调用（B4.7）。
#[doc(hidden)]
pub fn init_with_key(db_path: &Path, key: &str) -> Result<Db, AppError> {
    let mut conn = open_encrypted(db_path, key)?;
    apply_migrations(&mut conn, db_path)?;
    Ok(Db { conn: Mutex::new(conn) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_then_open_creates_vec0_table() {
        // B4.4 验收：注册后内存连接可直接建 vec0 虚拟表
        register_vec_extension().expect("注册");
        let conn = Connection::open_in_memory().expect("内存库");
        conn.execute_batch(
            "CREATE VIRTUAL TABLE v USING vec0 (embedding float[4], +source_text TEXT);",
        )
        .expect("vec0 建表");
        let version: String =
            conn.query_row("SELECT vec_version()", [], |row| row.get(0)).expect("vec_version");
        assert!(!version.is_empty());
    }
}
