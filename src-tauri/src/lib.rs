use serde::Serialize;
use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tauri::Manager;

#[derive(Debug, Serialize)]
pub struct AdbDevice {
    pub serial: String,
    pub state: String,
}

#[derive(Debug, Serialize)]
pub struct EscalationStep {
    pub name: String,
    pub ok: bool,
    pub output: String,
}

#[derive(Debug, Serialize)]
pub struct EscalationResult {
    pub success: bool,
    pub steps: Vec<EscalationStep>,
}

#[derive(Debug, Serialize)]
pub struct AdbCheck {
    pub path: String,
    pub version: String,
}

/// 内置 T5m 305 提权用的 preload.so（编译期内嵌，自包含）
const PRELOAD_SO: &[u8] = include_bytes!("../../src/assets/preload/enzymeym-t5m305-preload-dev.so");
/// Google 官方 platform-tools 下载地址（按平台选择）
const PLATFORM_TOOLS_URL_WIN: &str =
    "https://dl.google.com/android/repository/platform-tools-latest-windows.zip";
const PLATFORM_TOOLS_URL_LINUX: &str =
    "https://dl.google.com/android/repository/platform-tools-latest-linux.zip";

/// 当前平台的 platform-tools 下载地址
fn platform_tools_url() -> &'static str {
    if cfg!(target_os = "windows") {
        PLATFORM_TOOLS_URL_WIN
    } else {
        PLATFORM_TOOLS_URL_LINUX
    }
}

/// 当前平台的 adb 可执行文件名（Windows 为 adb.exe，其余平台为 adb）
fn adb_exe_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "adb.exe"
    } else {
        "adb"
    }
}

/// 在 PATH 中搜索可执行文件
fn search_in_path(bin: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let p = dir.join(bin);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

/// 搜索系统中 adb 的常见安装位置
fn find_adb() -> Option<PathBuf> {
    let exe = adb_exe_name();

    // 1. PATH 搜索（Linux 下 apt 安装的 adb 位于 /usr/bin）
    if let Some(p) = search_in_path(exe) {
        return Some(p);
    }

    let mut candidates: Vec<PathBuf> = Vec::new();

    // 环境变量指定（跨平台）
    for var in ["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
        if let Ok(p) = std::env::var(var) {
            candidates.push(PathBuf::from(p).join("platform-tools").join(exe));
        }
    }

    #[cfg(target_os = "windows")]
    {
        // Windows: Android SDK 默认路径与常见手动安装路径
        candidates.push(
            std::env::var("LOCALAPPDATA")
                .map(|p| PathBuf::from(p).join(r"Android\Sdk\platform-tools\adb.exe"))
                .unwrap_or_default(),
        );
        candidates.push(PathBuf::from(r"C:\adb\adb.exe"));
        candidates.push(PathBuf::from(r"C:\platform-tools\adb.exe"));
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Linux/macOS: 用户目录与发行版常见安装路径
        if let Some(home) = std::env::var_os("HOME") {
            let home = PathBuf::from(home);
            candidates.push(home.join("Android/Sdk/platform-tools").join(exe));
            candidates.push(home.join("android-sdk/platform-tools").join(exe));
        }
        candidates.push(PathBuf::from("/usr/lib/android-sdk/platform-tools").join(exe));
        candidates.push(PathBuf::from("/opt/android-sdk/platform-tools").join(exe));
        candidates.push(PathBuf::from("/usr/local/bin").join(exe));
    }

    for path in &candidates {
        if path.exists() {
            return Some(path.clone());
        }
    }

    None
}

/// 解析可用的 adb 路径：
/// 1. 用户显式指定（不存在则报错，不自动下载）
/// 2. 常见路径 / PATH 搜索
/// 3. 应用数据目录中已自动安装的 adb
/// 4. 自动下载安装 platform-tools
fn ensure_adb(app: &tauri::AppHandle, adb_path: Option<String>) -> Result<PathBuf, String> {
    if let Some(ref path) = adb_path {
        let p = PathBuf::from(path);
        if p.exists() {
            return Ok(p);
        }
        return Err(format!("指定的 adb 路径不存在: {}", path));
    }

    if let Some(p) = find_adb() {
        return Ok(p);
    }

    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("无法获取应用数据目录: {}", e))?;
    let adb_exe = dir.join("platform-tools").join(adb_exe_name());
    if adb_exe.exists() {
        return Ok(adb_exe);
    }

    install_platform_tools(&dir)?;
    Ok(adb_exe)
}

/// 下载并解压 platform-tools 到指定目录（跨平台：ureq 下载 + zip 解压）
fn install_platform_tools(dir: &PathBuf) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("创建目录失败: {}", e))?;

    let zip_path = dir.join("platform-tools.zip");

    // 1. 下载
    let resp = ureq::get(platform_tools_url())
        .call()
        .map_err(|e| format!("自动下载 adb 套件失败（请检查网络）: {}", e))?;
    let mut file = std::fs::File::create(&zip_path)
        .map_err(|e| format!("创建下载文件失败: {}", e))?;
    std::io::copy(&mut resp.into_reader(), &mut file)
        .map_err(|e| format!("写入下载文件失败: {}", e))?;
    drop(file);

    // 2. 解压（zip 内顶层目录即 platform-tools）
    let file = std::fs::File::open(&zip_path)
        .map_err(|e| format!("打开下载文件失败: {}", e))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("读取 adb 套件压缩包失败: {}", e))?;
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("读取压缩包条目失败: {}", e))?;
        let outpath = match entry.enclosed_name() {
            Some(p) => dir.join(p),
            None => continue,
        };
        if entry.is_dir() {
            std::fs::create_dir_all(&outpath).map_err(|e| format!("创建目录失败: {}", e))?;
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {}", e))?;
            }
            let mut out = std::fs::File::create(&outpath)
                .map_err(|e| format!("写入文件失败: {}", e))?;
            std::io::copy(&mut entry, &mut out)
                .map_err(|e| format!("解压文件失败: {}", e))?;
            drop(out);
            // Linux/macOS：还原 zip 中的 unix 权限（否则解压出的 adb 不可执行）
            #[cfg(not(target_os = "windows"))]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = entry.unix_mode() {
                    let _ = std::fs::set_permissions(
                        &outpath,
                        std::fs::Permissions::from_mode(mode),
                    );
                }
            }
        }
    }
    drop(archive);
    let _ = std::fs::remove_file(&zip_path);

    if !dir.join("platform-tools").join(adb_exe_name()).exists() {
        return Err("自动安装 adb 套件失败，请手动设置 adb 路径后重试".to_string());
    }
    Ok(())
}

/// 执行命令并返回 (是否成功, 合并输出)
fn run_command(cmd: &mut Command) -> Result<(bool, String), String> {
    let output = cmd
        .output()
        .map_err(|e| format!("执行 adb 失败 ({}): {}", cmd.get_program().to_string_lossy(), e))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let text = if stdout.trim().is_empty() { stderr } else { stdout };
    Ok((output.status.success(), text.trim().to_string()))
}

/// 带超时执行命令（超时则 kill），用于可能挂起的注入命令
fn run_with_timeout(cmd: &mut Command, secs: u64) -> Result<(bool, String), String> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::null());
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("启动 adb 失败 ({}): {}", cmd.get_program().to_string_lossy(), e))?;

    let start = Instant::now();
    loop {
        match child.try_wait().map_err(|e| format!("等待 adb 失败: {}", e))? {
            Some(status) => {
                let mut out = String::new();
                if let Some(mut s) = child.stdout.take() {
                    let _ = s.read_to_string(&mut out);
                }
                return Ok((status.success(), out.trim().to_string()));
            }
            None => {
                if start.elapsed() >= Duration::from_secs(secs) {
                    let _ = child.kill();
                    let mut out = String::new();
                    if let Some(mut s) = child.stdout.take() {
                        let _ = s.read_to_string(&mut out);
                    }
                    let _ = child.wait();
                    return Ok((false, format!("命令超时（{} 秒）已终止: {}", secs, out.trim())));
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

/// 构造带 -s 参数的 adb 命令
fn adb_cmd(adb: &PathBuf, serial: &str) -> Command {
    let mut c = Command::new(adb);
    c.arg("-s").arg(serial);
    c
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn get_adb_devices(
    app: tauri::AppHandle,
    adb_path: Option<String>,
) -> Result<Vec<AdbDevice>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let adb = ensure_adb(&app, adb_path)?;

        let output = Command::new(&adb)
            .arg("devices")
            .output()
            .map_err(|e| format!("执行 adb 失败 ({}): {}", adb.display(), e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("adb 命令失败: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut devices = Vec::new();

        for line in stdout.lines().skip(1) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                devices.push(AdbDevice {
                    serial: parts[0].to_string(),
                    state: parts[1].to_string(),
                });
            }
        }

        Ok(devices)
    })
    .await
    .map_err(|e| format!("设备扫描任务异常: {}", e))?
}

/// 检查 adb 安装情况：解析 adb 路径并获取版本信息（必要时自动下载安装）
#[tauri::command]
async fn check_adb_installation(
    app: tauri::AppHandle,
    adb_path: Option<String>,
) -> Result<AdbCheck, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let adb = ensure_adb(&app, adb_path)?;

        let output = Command::new(&adb)
            .arg("version")
            .output()
            .map_err(|e| format!("执行 adb 失败 ({}): {}", adb.display(), e))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("adb 版本命令失败: {}", stderr));
        }

        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(AdbCheck {
            path: adb.display().to_string(),
            version,
        })
    })
    .await
    .map_err(|e| format!("adb 检查任务异常: {}", e))?
}

/// 通过系统保存对话框导出日志文件，返回保存路径（取消则返回 Err("已取消导出")）
#[tauri::command]
async fn export_logs(content: String, default_name: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = rfd::FileDialog::new()
            .set_file_name(&default_name)
            .save_file()
            .ok_or_else(|| "已取消导出".to_string())?;
        std::fs::write(&path, content).map_err(|e| format!("写入日志文件失败: {}", e))?;
        Ok(path.display().to_string())
    })
    .await
    .map_err(|e| format!("导出任务异常: {}", e))?
}

/// 对指定设备执行 preload.so 提权流程：push → chmod → LD_PRELOAD 注入 → su 验证
#[tauri::command]
async fn escalate_privileges(
    app: tauri::AppHandle,
    adb_path: Option<String>,
    serial: String,
) -> Result<EscalationResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let adb = ensure_adb(&app, adb_path)?;
        let mut steps = Vec::new();

        // 将内置 preload.so 写入临时文件供 adb push
        let tmp_path = std::env::temp_dir().join("t5m_preload.so");
        std::fs::write(&tmp_path, PRELOAD_SO)
            .map_err(|e| format!("写入临时 preload.so 失败: {}", e))?;

        // 1. 推送
        let mut push = adb_cmd(&adb, &serial);
        push.args([
            "push",
            tmp_path.to_str().unwrap_or(""),
            "/data/local/tmp/preload.so",
        ]);
        let (ok, out) = run_command(&mut push)?;
        steps.push(EscalationStep {
            name: "push preload.so".to_string(),
            ok,
            output: out,
        });

        // 2. 赋予执行权限
        let mut chmod = adb_cmd(&adb, &serial);
        chmod.args(["shell", "chmod 755 /data/local/tmp/preload.so"]);
        let (ok, out) = run_command(&mut chmod)?;
        steps.push(EscalationStep {
            name: "chmod 755".to_string(),
            ok,
            output: out,
        });

        // 3. 通过 app_process 注入（方式 A，可能挂起，带 8 秒超时；失败不中断流程）
        let mut inject = adb_cmd(&adb, &serial);
        inject.args([
            "shell",
            "LD_PRELOAD=/data/local/tmp/preload.so /system/bin/app_process /system/bin com.android.commands.am.Am",
        ]);
        let (ok, out) = run_with_timeout(&mut inject, 8)
            .unwrap_or((false, "注入命令执行异常".to_string()));
        steps.push(EscalationStep {
            name: "LD_PRELOAD 注入".to_string(),
            ok,
            output: out,
        });

        // 清理临时文件
        let _ = std::fs::remove_file(&tmp_path);

        // 4. 验证提权结果
        let mut verify = adb_cmd(&adb, &serial);
        verify.args(["shell", "/apex/com.android.virt/bin/su -c 'id'"]);
        let (ok, out) = run_command(&mut verify)?;
        let success = ok && out.contains("uid=0(root)");
        steps.push(EscalationStep {
            name: "验证 su".to_string(),
            ok: success,
            output: out,
        });

        Ok(EscalationResult { success, steps })
    })
    .await
    .map_err(|e| format!("提权任务异常: {}", e))?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            get_adb_devices,
            check_adb_installation,
            escalate_privileges,
            export_logs
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
