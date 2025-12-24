// src/main.rs - FINAL FIX: Registry + Direct Launch + File Monitoring
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use std::collections::HashMap;
use dunce;

#[cfg(not(target_os = "windows"))]
use std::process::Command;

use arboard::Clipboard;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
struct AppGeometry {
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    auto_position: bool,
}

#[derive(Debug)]
struct Config {
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    auto_position: bool,
    default_app: Option<String>,
    backup_dir: PathBuf,
    pause_behavior: PauseBehavior,
    editor_apps: Vec<String>,
    viewer_apps: Vec<String>,
    always_pause_apps: Vec<String>,
    // NEW: Per-app geometry configurations
    app_geometries: std::collections::HashMap<String, AppGeometry>,
    #[cfg(target_os = "windows")]
    terminal: TerminalType,
}

#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Copy, PartialEq)]
enum TerminalType {
    Cmd,
    WindowsTerminal,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum PauseBehavior {
    Never,
    Always,
    Auto,
}

#[cfg(target_os = "windows")]
impl Default for TerminalType {
    fn default() -> Self {
        TerminalType::Cmd
    }
}

impl Default for Config {
    fn default() -> Self {
        let backup_dir = if let Ok(exe_path) = env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                exe_dir.join("backups")
            } else {
                PathBuf::from("backups")
            }
        } else {
            PathBuf::from("backups")
        };

                Config {
            width: 800,
            height: 600,
            x: 100,
            y: 100,
            auto_position: false,
            default_app: None,
            backup_dir,
            pause_behavior: PauseBehavior::Auto,
            editor_apps: vec![
                "vim".to_string(), "nvim".to_string(), "nano".to_string(),
                "emacs".to_string(), "micro".to_string(), "helix".to_string(),
                "hx".to_string(), "code".to_string(), "subl".to_string(),
            ],
            viewer_apps: vec![
                "bat".to_string(), "less".to_string(), "more".to_string(),
                "cat".to_string(), "type".to_string(),
            ],
            always_pause_apps: vec![
                "python".to_string(), "python3".to_string(), "node".to_string(),
                "ruby".to_string(), "perl".to_string(),
            ],
            app_geometries: HashMap::new(),
            #[cfg(target_os = "windows")]
            terminal: TerminalType::default(),
        }
    }
}

#[derive(Debug, Default)]
struct RunOptions {
    always_on_top: bool,
    use_clipboard: bool,
    output_file: Option<PathBuf>,
    backup_dir: Option<PathBuf>,
}

impl Config {
    fn parse(content: &str) -> Self {
        let mut config = Config::default();
        let mut current_section: Option<String> = None;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Check for section headers like [bat]
            if line.starts_with('[') && line.ends_with(']') {
                let section = line[1..line.len()-1].to_string();
                if section == "terminal" {
                    current_section = None;
                } else {
                    current_section = Some(section);
                }
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"').trim_matches('\'');

                // If we're in a per-app section, handle geometry
                if let Some(ref app_name) = current_section {
                    let geometry = config.app_geometries.entry(app_name.clone())
                        .or_insert(AppGeometry {
                            width: config.width,
                            height: config.height,
                            x: config.x,
                            y: config.y,
                            auto_position: config.auto_position,
                        });

                    match key {
                        "width" => {
                            if let Ok(v) = value.parse() {
                                geometry.width = v;
                            }
                        }
                        "height" => {
                            if let Ok(v) = value.parse() {
                                geometry.height = v;
                            }
                        }
                        "x" => {
                            if let Ok(v) = value.parse() {
                                geometry.x = v;
                            }
                        }
                        "y" => {
                            if let Ok(v) = value.parse() {
                                geometry.y = v;
                            }
                        }
                        "auto_position" => {
                            geometry.auto_position = matches!(value.to_lowercase().as_str(), "true" | "1" | "yes");
                        }
                        _ => {}
                    }
                    continue;
                }

                // Global settings (in [terminal] section or no section)
                match key {
                    "width" => {
                        if let Ok(v) = value.parse() {
                            config.width = v;
                        }
                    }
                    "height" => {
                        if let Ok(v) = value.parse() {
                            config.height = v;
                        }
                    }
                    "x" => {
                        if let Ok(v) = value.parse() {
                            config.x = v;
                        }
                    }
                    "y" => {
                        if let Ok(v) = value.parse() {
                            config.y = v;
                        }
                    }
                    "auto_position" => {
                        config.auto_position = matches!(value.to_lowercase().as_str(), "true" | "1" | "yes");
                    }
                    "pause_behavior" => {
                        config.pause_behavior = match value.to_lowercase().as_str() {
                            "never" => PauseBehavior::Never,
                            "always" => PauseBehavior::Always,
                            "auto" => PauseBehavior::Auto,
                            _ => PauseBehavior::Never,
                        };
                    }
                    "default_app" => {
                        if !value.is_empty() {
                            config.default_app = Some(value.to_string());
                        }
                    }
                    "backup_dir" => {
                        if !value.is_empty() {
                            config.backup_dir = PathBuf::from(value);
                        }
                    }
                    "editor_apps" => {
                        config.editor_apps = value
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                    "viewer_apps" => {
                        config.viewer_apps = value
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                    "always_pause_apps" => {
                        config.always_pause_apps = value
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                    #[cfg(target_os = "windows")]
                    "terminal" => {
                        config.terminal = match value.to_lowercase().as_str() {
                            "wt" | "wt.exe" | "windows_terminal" | "windowsterminal" => {
                                TerminalType::WindowsTerminal
                            }
                            _ => TerminalType::Cmd,
                        };
                    }
                    _ => {}
                }
            }
        }

        config
    }

    // Get geometry for specific app, fallback to default
    fn get_geometry(&self, app: &str) -> AppGeometry {
        let app_lower = app.to_lowercase();
        let app_first_word = app_lower.split_whitespace().next().unwrap_or("");

        // Try to find app-specific geometry
        if let Some(geom) = self.app_geometries.get(app_first_word) {
            geom.clone()
        } else {
            // Fallback to default
            AppGeometry {
                width: self.width,
                height: self.height,
                x: self.x,
                y: self.y,
                auto_position: self.auto_position,
            }
        }
    }
}

fn get_config_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = env::var("APPDATA") {
            return PathBuf::from(appdata).join("rund");
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = env::var("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("rund");
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(xdg_config) = env::var("XDG_CONFIG_HOME") {
            return PathBuf::from(xdg_config).join("rund");
        }
        if let Ok(home) = env::var("HOME") {
            return PathBuf::from(home).join(".config").join("rund");
        }
    }

    PathBuf::from(".")
}

fn get_config_path() -> PathBuf {
    if let Ok(exe_path) = env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let config_path = exe_dir.join("config.toml");
            if config_path.exists() {
                return config_path;
            }
            if let Ok(test_file) = fs::File::create(exe_dir.join(".rund_write_test")) {
                drop(test_file);
                fs::remove_file(exe_dir.join(".rund_write_test")).ok();
                return config_path;
            }
        }
    }

    let config_dir = get_config_dir();
    fs::create_dir_all(&config_dir).ok();
    config_dir.join("config.toml")
}

fn load_config() -> io::Result<Config> {
    let config_path = get_config_path();

    if !config_path.exists() {
        #[cfg(target_os = "windows")]
        let default_config = r#"# rund configuration file

[terminal]
width = 800
height = 600

# Position settings
# If auto_position = true, let Windows decide position
# If auto_position = false, use x and y below
auto_position = false
x = 100
y = 100

# Terminal to use: "cmd" or "wt" (Windows Terminal)
terminal = "cmd"

# Pause behavior after command execution:
# "never"  - No pause, window closes immediately
# "always" - Always pause with "Press any key..."
# "auto"   - Smart detection based on app lists below
pause_behavior = "auto"

# App classifications for smart pause behavior
# Editors: NEVER pause (they're interactive)
editor_apps = "vim, nvim, nano, emacs, micro, helix, hx, code, subl"

# Viewers: Pause ONLY for small files (<30 lines)
viewer_apps = "bat, less, more, cat, type"

# Always pause: For scripts/interpreters that produce output
always_pause_apps = "python, python3, node, ruby, perl, php"

# Directory for backup files (default: ./backups)
backup_dir = "backups"

# Uncomment to set default app
# default_app = "nvim"

# Per-app geometry configuration (optional)
# Uncomment and customize for specific apps
# These settings override the default [terminal] geometry

#[bat]
#width = 1200
#height = 800
#x = 200
#y = 150
#auto_position = false

#[nvim]
#width = 1000
#height = 700
#x = 100
#y = 100
#auto_position = true

#[python]
#width = 900
#height = 600
"#;

        #[cfg(not(target_os = "windows"))]
        let default_config = r#"# rund configuration file

[terminal]
width = 800
height = 600

# Position settings
auto_position = false
x = 100
y = 100

# Pause behavior: "never", "always", "auto"
pause_behavior = "auto"

# App classifications for smart pause behavior
# Editors: NEVER pause (they're interactive)
editor_apps = "vim, nvim, nano, emacs, micro, helix, hx, code, subl"

# Viewers: Pause ONLY for small files (<30 lines)
viewer_apps = "bat, less, more, cat"

# Always pause: For scripts/interpreters that produce output
always_pause_apps = "python, python3, node, ruby, perl, php"

# Directory for backup files (default: ./backups)
backup_dir = "backups"

# Uncomment to set default app
# default_app = "nvim"
"#;

        fs::write(&config_path, default_config)?;
    }

    let content = fs::read_to_string(&config_path)?;
    Ok(Config::parse(&content))
}

fn calculate_file_hash(path: &PathBuf) -> io::Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn create_backup(file_path: &PathBuf, backup_dir: &PathBuf) -> io::Result<PathBuf> {
    fs::create_dir_all(backup_dir)?;

    let file_name = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");

    let file_ext = file_path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| format!(".{}", s))
        .unwrap_or_default();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let timestamp = now.as_secs();

    let backup_name = format!("{}_{}{}", file_name, timestamp, file_ext);
    let backup_path = backup_dir.join(backup_name);

    fs::copy(file_path, &backup_path)?;

    Ok(backup_path)
}

#[cfg(target_os = "windows")]
mod windows {
    use super::TerminalType;
    use std::ffi::OsStr;
    use std::io;
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;
    use std::path::PathBuf;
    use std::ptr;

    type HWND = *mut std::ffi::c_void;
    type LPCWSTR = *const u16;
    type UINT = u32;
    type HANDLE = *mut std::ffi::c_void;
    type DWORD = u32;
    type LPVOID = *mut std::ffi::c_void;
    type BOOL = i32;
    type WORD = u16;
    type LPWSTR = *mut u16;
    type HKEY = *mut std::ffi::c_void;
    type LPCSTR = *const u8;
    type LPDWORD = *mut DWORD;
    type LPBYTE = *mut u8;
    type LONG = i32;
    type REGSAM = u32;

    // Wrapper untuk HANDLE agar bisa Send + Sync
    #[derive(Debug)]
    pub struct ProcessHandle(HANDLE);
    
    unsafe impl Send for ProcessHandle {}
    unsafe impl Sync for ProcessHandle {}
    
    impl ProcessHandle {
        pub fn new(handle: HANDLE) -> Self {
            ProcessHandle(handle)
        }
        
        pub fn is_null(&self) -> bool {
            self.0.is_null()
        }
        
        pub fn as_raw(&self) -> HANDLE {
            self.0
        }
    }

    #[repr(C)]
    struct STARTUPINFOW {
        cb: DWORD,
        lp_reserved: LPWSTR,
        lp_desktop: LPWSTR,
        lp_title: LPWSTR,
        dw_x: DWORD,
        dw_y: DWORD,
        dw_x_size: DWORD,
        dw_y_size: DWORD,
        dw_x_count_chars: DWORD,
        dw_y_count_chars: DWORD,
        dw_fill_attribute: DWORD,
        dw_flags: DWORD,
        w_show_window: WORD,
        cb_reserved2: WORD,
        lp_reserved2: *mut u8,
        h_std_input: HANDLE,
        h_std_output: HANDLE,
        h_std_error: HANDLE,
    }

    #[repr(C)]
    struct PROCESS_INFORMATION {
        h_process: HANDLE,
        h_thread: HANDLE,
        dw_process_id: DWORD,
        dw_thread_id: DWORD,
    }

    const CREATE_NEW_CONSOLE: DWORD = 0x00000010;
    const MB_OK: UINT = 0x00000000;
    const MB_ICONERROR: UINT = 0x00000010;
    const MB_TASKMODAL: UINT = 0x00002000;
    const INFINITE: DWORD = 0xFFFFFFFF;
    
    const HKEY_CURRENT_USER: HKEY = 0x80000001 as HKEY;
    const KEY_WRITE: REGSAM = 0x20006;
    const REG_DWORD: DWORD = 4;

    #[link(name = "user32")]
    extern "system" {
        fn MessageBoxW(hwnd: HWND, text: LPCWSTR, caption: LPCWSTR, utype: UINT) -> i32;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn CreateProcessW(
            application_name: LPCWSTR,
            command_line: LPWSTR,
            process_attributes: LPVOID,
            thread_attributes: LPVOID,
            inherit_handles: BOOL,
            creation_flags: DWORD,
            environment: LPVOID,
            current_directory: LPCWSTR,
            startup_info: *mut STARTUPINFOW,
            process_information: *mut PROCESS_INFORMATION,
        ) -> BOOL;
        fn CloseHandle(object: HANDLE) -> BOOL;
        fn WaitForSingleObject(handle: HANDLE, milliseconds: DWORD) -> DWORD;
    }

    #[link(name = "advapi32")]
    extern "system" {
        fn RegCreateKeyExA(
            hkey: HKEY,
            lpsubkey: LPCSTR,
            reserved: DWORD,
            lpclass: LPWSTR,
            dwoptions: DWORD,
            samdesired: REGSAM,
            lpsecurityattributes: LPVOID,
            phkresult: *mut HKEY,
            lpdwdisposition: LPDWORD,
        ) -> LONG;
        fn RegSetValueExA(
            hkey: HKEY,
            lpvaluename: LPCSTR,
            reserved: DWORD,
            dwtype: DWORD,
            lpdata: LPBYTE,
            cbdata: DWORD,
        ) -> LONG;
        fn RegDeleteValueA(hkey: HKEY, lpvaluename: LPCSTR) -> LONG;
        fn RegCloseKey(hkey: HKEY) -> LONG;
    }

    fn to_wide_string(s: &str) -> Vec<u16> {
        OsStr::new(s).encode_wide().chain(once(0)).collect()
    }

    pub fn show_error_centered(msg: &str) {
        let title = to_wide_string("rund - Error");
        let text = to_wide_string(msg);

        unsafe {
            MessageBoxW(
                ptr::null_mut(),
                text.as_ptr(),
                title.as_ptr(),
                MB_OK | MB_ICONERROR | MB_TASKMODAL,
            );
        }
    }

    fn set_console_registry_by_title(title: &str, x: i32, y: i32, width: u32, height: u32, auto_pos: bool) -> io::Result<()> {
        unsafe {
            let key_path_str = format!("Console\\{}\0", title);
            let key_path = key_path_str.as_bytes();
            let mut hkey: HKEY = ptr::null_mut();
            let mut disposition: DWORD = 0;

            let result = RegCreateKeyExA(
                HKEY_CURRENT_USER,
                key_path.as_ptr(),
                0,
                ptr::null_mut(),
                0,
                KEY_WRITE,
                ptr::null_mut(),
                &mut hkey,
                &mut disposition,
            );

            if result != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Failed to create registry key: {}", result),
                ));
            }

            if !auto_pos {
                let pos: DWORD = ((y as DWORD) << 16) | (x as DWORD & 0xFFFF);
                RegSetValueExA(
                    hkey,
                    b"WindowPosition\0".as_ptr(),
                    0,
                    REG_DWORD,
                    &pos as *const DWORD as LPBYTE,
                    4,
                );

                let cols = width / 8;
                let rows = height / 16;
                let size: DWORD = (rows << 16) | cols;
                RegSetValueExA(
                    hkey,
                    b"WindowSize\0".as_ptr(),
                    0,
                    REG_DWORD,
                    &size as *const DWORD as LPBYTE,
                    4,
                );
            } else {
                RegDeleteValueA(hkey, b"WindowPosition\0".as_ptr());
                RegDeleteValueA(hkey, b"WindowSize\0".as_ptr());
            }

            RegCloseKey(hkey);
        }

        Ok(())
    }

    pub fn run_and_wait(
        app: &str,
        file_path: &Option<PathBuf>,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        auto_position: bool,
        terminal_type: TerminalType,
        no_pause: bool,
    ) -> io::Result<ProcessHandle> {
        match terminal_type {
            TerminalType::Cmd => run_cmd_direct(app, file_path, x, y, width, height, auto_position, no_pause),
            TerminalType::WindowsTerminal => run_wt(app, file_path, x, y, width, height, auto_position, no_pause),
        }
    }

    fn run_cmd_direct(
        app: &str,
        file_path: &Option<PathBuf>,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        auto_position: bool,
        no_pause: bool,
    ) -> io::Result<ProcessHandle> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let window_title = format!("rund_{}", timestamp);

        set_console_registry_by_title(&window_title, x, y, width, height, auto_position)?;

        let cmd_path = to_wide_string("C:\\Windows\\System32\\cmd.exe");
        
        // PERBAIKAN CRITICAL:
        // SELALU gunakan /C agar terminal AUTO-CLOSE setelah selesai
        // Jika butuh pause (file kecil), tambahkan pause TAPI TETAP /C
        let full_cmd = if no_pause {
            // No pause - langsung close setelah app exit
            if let Some(ref path) = file_path {
                format!("/C title {} & {} \"{}\"", window_title, app, path.display())
            } else {
                format!("/C title {} & {}", window_title, app)
            }
        } else {
            // With pause - tapi tetap /C jadi close setelah user press key
            if let Some(ref path) = file_path {
                format!("/C title {} & {} \"{}\" & pause", window_title, app, path.display())
            } else {
                format!("/C title {} & {} & pause", window_title, app)
            }
        };
        
        let mut cmd_line = to_wide_string(&full_cmd);

        let mut si: STARTUPINFOW = unsafe { std::mem::zeroed() };
        si.cb = std::mem::size_of::<STARTUPINFOW>() as DWORD;

        let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

        let result = unsafe {
            CreateProcessW(
                cmd_path.as_ptr(),
                cmd_line.as_mut_ptr(),
                ptr::null_mut(),
                ptr::null_mut(),
                0,
                CREATE_NEW_CONSOLE,
                ptr::null_mut(),
                ptr::null(),
                &mut si,
                &mut pi,
            )
        };

        if result == 0 {
            let error = io::Error::last_os_error();
            return Err(error);
        }

        unsafe {
            CloseHandle(pi.h_thread);
        }

        Ok(ProcessHandle::new(pi.h_process))
    }

    fn run_wt(
        app: &str,
        file_path: &Option<PathBuf>,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        auto_position: bool,
        no_pause: bool,
    ) -> io::Result<ProcessHandle> {
        use std::process::Command;

        let cols = width / 9;
        let rows = height / 19;

        let cmd_to_run = if no_pause {
            if let Some(ref path) = file_path {
                format!("{} \"{}\"", app, path.display())
            } else {
                app.to_string()
            }
        } else {
            if let Some(ref path) = file_path {
                format!("{} \"{}\" & pause", app, path.display())
            } else {
                format!("{} & pause", app)
            }
        };

        // Windows Terminal command line
        let mut wt_args = vec![];
        
        // Add position and size ONLY if not auto-position
        // If auto-position, don't specify --pos or --size, let wt decide
        if !auto_position {
            wt_args.push("--pos".to_string());
            wt_args.push(format!("{},{}", x, y));
            wt_args.push("--size".to_string());
            wt_args.push(format!("{},{}", cols, rows));
        }
        
        wt_args.push("--title".to_string());
        wt_args.push("rund".to_string());
        wt_args.push("cmd.exe".to_string());
        wt_args.push("/C".to_string());
        wt_args.push(cmd_to_run);

        Command::new("wt.exe").args(&wt_args).spawn().map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to launch Windows Terminal: {}", e),
            )
        })?;

        Ok(ProcessHandle::new(ptr::null_mut()))
    }

    pub fn wait_for_process(handle: &ProcessHandle) {
        if !handle.is_null() {
            unsafe {
                WaitForSingleObject(handle.as_raw(), INFINITE);
                CloseHandle(handle.as_raw());
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn show_error(msg: &str) {
    #[cfg(target_os = "macos")]
    {
        Command::new("osascript")
            .args(&[
                "-e",
                &format!(
                    r#"display dialog "{}" with title "rund - Error" buttons {{"OK"}} default button "OK" with icon stop"#,
                    msg.replace('"', "\\\"")
                ),
            ])
            .output()
            .ok();
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("zenity")
            .args(&[
                "--error",
                "--title=rund - Error",
                &format!("--text={}", msg),
                "--width=400",
            ])
            .output()
            .or_else(|_| {
                Command::new("kdialog")
                    .args(&["--error", msg, "--title", "rund - Error"])
                    .output()
            })
            .or_else(|_| {
                Command::new("notify-send")
                    .args(&["-u", "critical", "rund - Error", msg])
                    .output()
            })
            .ok();
    }

    eprintln!("Error: {}", msg);
}

#[cfg(target_os = "windows")]
fn show_error(msg: &str) {
    windows::show_error_centered(msg);
    eprintln!("Error: {}", msg);
}

fn run_in_terminal(app: &str, config: &Config, options: &RunOptions) -> io::Result<()> {
    let backup_dir = options
        .backup_dir
        .as_ref()
        .unwrap_or(&config.backup_dir)
        .clone();

    let (file_path, initial_hash) = if options.use_clipboard || options.output_file.is_some() {
        let file_path = if let Some(ref output) = options.output_file {
            let path = output.clone();

            if options.use_clipboard {
                let mut clipboard = Clipboard::new().map_err(|e| {
                    io::Error::new(io::ErrorKind::Other, format!("Clipboard error: {}", e))
                })?;

                let content = clipboard.get_text().map_err(|e| {
                    io::Error::new(io::ErrorKind::Other, format!("Clipboard error: {}", e))
                })?;

                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }

                fs::write(&path, content)?;
            }

            path
        } else {
            let mut clipboard = Clipboard::new().map_err(|e| {
                io::Error::new(io::ErrorKind::Other, format!("Clipboard error: {}", e))
            })?;

            let content = clipboard.get_text().map_err(|e| {
                io::Error::new(io::ErrorKind::Other, format!("Clipboard error: {}", e))
            })?;

            let temp_dir = env::temp_dir();
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let temp_file = temp_dir.join(format!("rund_clipboard_{}.txt", timestamp));

            fs::write(&temp_file, content)?;
            temp_file
        };

        let initial_hash = if file_path.exists() {
            calculate_file_hash(&file_path)?
        } else {
            String::new()
        };

        (Some(file_path), initial_hash)
    } else {
        (None, String::new())
    };

    // Detect app type dari command name - GUNAKAN CONFIG!
    let app_lower = app.to_lowercase();
    let app_first_word = app_lower.split_whitespace().next().unwrap_or("");
    
    // Check against config lists
    let is_editor = config.editor_apps.iter()
        .any(|e| app_first_word.contains(&e.to_lowercase()) || app_first_word == e.to_lowercase());
    
    let is_viewer = config.viewer_apps.iter()
        .any(|v| app_first_word.contains(&v.to_lowercase()) || app_first_word == v.to_lowercase());
    
    let is_always_pause = config.always_pause_apps.iter()
        .any(|a| app_first_word.contains(&a.to_lowercase()) || app_first_word == a.to_lowercase());
    
    // Special case: 'type' command needs '| more' for large files!
    let is_type_command = app_first_word == "type";

    // SMART PAUSE DETECTION:
    // - Editors: NEVER need pause (mereka interactive)
    // - Viewers: Need pause ONLY for small files (< 30 lines)
    // - Always pause apps: ALWAYS pause
    // - Unknown: Pause by default (safe)
    let needs_pause = if is_editor {
        // Editors never need pause - they're interactive!
        false
    } else if is_always_pause {
        // Apps explicitly marked to always pause
        true
    } else if is_viewer {
        // Viewers: check file size
        if let Some(ref path) = file_path {
            if path.exists() {
                match fs::read_to_string(path) {
                    Ok(content) => {
                        let line_count = content.lines().count();
                        // Small file needs pause to see output
                        line_count < 30
                    }
                    Err(_) => false,
                }
            } else {
                false
            }
        } else {
            false
        }
    } else {
        // Unknown commands - pause by default for safety
        true
    };

    // Override with config if explicitly set
    let no_pause = match config.pause_behavior {
        PauseBehavior::Never => true,
        PauseBehavior::Always => false,
        PauseBehavior::Auto => !needs_pause, // Smart detection!
    };
    
    // Modify app command for 'type' with large files
    let final_app = if is_type_command && !needs_pause {
        // Large file with type command - add '| more' for paging
        format!("{} | more", app)
    } else {
        app.to_string()
    };

    // Get geometry for this specific app (with fallback to default)
    let geom = config.get_geometry(app);

    #[cfg(target_os = "windows")]
    {
        let process_handle = windows::run_and_wait(
            &final_app,
            &file_path,
            geom.x,
            geom.y,
            geom.width,
            geom.height,
            geom.auto_position,
            config.terminal,
            no_pause,
        )?;

        if let Some(ref path) = file_path {
            if !initial_hash.is_empty() {
                let path_clone = path.clone();
                let backup_dir_clone = backup_dir.clone();
                let initial_hash_clone = initial_hash.clone();

                thread::spawn(move || {
                    windows::wait_for_process(&process_handle);
                    thread::sleep(Duration::from_millis(500));

                    if path_clone.exists() {
                        if let Ok(final_hash) = calculate_file_hash(&path_clone) {
                            if final_hash != initial_hash_clone {
                                if let Ok(backup_path) = create_backup(&path_clone, &backup_dir_clone) {
                                    println!("Backup created: {}", backup_path.display());
                                }
                            }
                        }
                    }
                });
            } else {
                windows::wait_for_process(&process_handle);
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let file_arg = if let Some(ref path) = file_path {
            format!(" \\\"{}\\\"", path.display())
        } else {
            String::new()
        };

        let pause_cmd = if no_pause { "" } else { "; read -p 'Press Enter to exit...'" };
        let script = format!(
            r#"tell application "Terminal"
    activate
    do script "{}{}{}; exit"
    set bounds of front window to {{{}, {}, {}, {}}}
end tell"#,
            app.replace('"', "\\\""),
            file_arg,
            pause_cmd,
            geom.x,
            geom.y,
            geom.x + geom.width as i32,
            geom.y + geom.height as i32
        );

        Command::new("osascript").arg("-e").arg(&script).spawn()?;
    }

    #[cfg(target_os = "linux")]
    {
        let file_arg = if let Some(ref path) = file_path {
            format!(" \"{}\"", path.display())
        } else {
            String::new()
        };

        let pause_cmd = if no_pause { "" } else { "; read -p 'Press Enter to exit...'" };
        let cmd_with_pause = format!("{}{}{}", app, file_arg, pause_cmd);

        let terminals = [
            (
                "alacritty",
                vec![
                    "--option",
                    &format!("window.dimensions.columns={}", geom.width / 8),
                    "--option",
                    &format!("window.dimensions.lines={}", geom.height / 16),
                    "--option",
                    &format!("window.position.x={}", geom.x),
                    "--option",
                    &format!("window.position.y={}", geom.y),
                    "-e",
                    "bash",
                    "-c",
                    &cmd_with_pause,
                ],
            ),
            (
                "kitty",
                vec![
                    "-o",
                    &format!("initial_window_width={}c", geom.width / 8),
                    "-o",
                    &format!("initial_window_height={}c", geom.height / 16),
                    "bash",
                    "-c",
                    &cmd_with_pause,
                ],
            ),
            ("gnome-terminal", vec!["--", "bash", "-c", &cmd_with_pause]),
            ("konsole", vec!["-e", "bash", "-c", &cmd_with_pause]),
            ("xterm", vec!["-e", "bash", "-c", &cmd_with_pause]),
        ];

        for (term, args) in &terminals {
            if Command::new(term).args(args).spawn().is_ok() {
                return Ok(());
            }
        }

        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "No supported terminal found. Please install: alacritty, kitty, gnome-terminal, konsole, or xterm",
        ));
    }

    Ok(())
}

fn print_help() {
    #[cfg(target_os = "windows")]
    let terminal_info = r#"
TERMINAL SELECTION (Windows only):
    terminal = "cmd" or terminal = "wt"
"#;

    #[cfg(not(target_os = "windows"))]
    let terminal_info = "";

    println!(
        r#"rund - Run CLI apps in detached terminal popup

USAGE:
    rund [OPTIONS] [APP] [ARGS...]
    rund --config
    rund --help

OPTIONS:
    -t, --top           Always-on-top (macOS/Linux only)
    -c, --clipboard     Read clipboard to file
    -o, --output FILE   Specify output file path
    -b, --backup DIR    Override backup directory
    --config            Show config file path
    -h, --help          Show this help

EXAMPLES:
    rund nvim file.txt
    rund -c -o c:\temp\test.py bat
    rund "python -m rich.emoji"
{}
CONFIG: {}

    width = 800
    height = 600
    auto_position = false
    x = 100
    y = 100
    terminal = "cmd"
    pause_behavior = "auto"
    
    # Customize app behavior:
    editor_apps = "vim, nvim, nano, emacs, micro, helix, hx, code, subl"
    viewer_apps = "bat, less, more, cat, type"
    always_pause_apps = "python, python3, node, ruby, perl, php"
    
    backup_dir = "backups"
"#,
        terminal_info,
        get_config_path().display()
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut options = RunOptions::default();
    let mut app_name: Option<String> = None;
    let mut app_args: Vec<String> = Vec::new();
    let mut i = 1;

    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                print_help();
                return;
            }
            "--config" => {
                println!("Config file: {}", get_config_path().display());
                return;
            }
            "-t" | "--top" => {
                options.always_on_top = true;
                i += 1;
            }
            "-c" | "--clipboard" => {
                options.use_clipboard = true;
                i += 1;
            }
            "-o" | "--output" => {
                if i + 1 < args.len() {
                    options.output_file = Some(PathBuf::from(&args[i + 1]));
                    i += 2;
                } else {
                    show_error("-o/--output requires a file path");
                    std::process::exit(1);
                }
            }
            "-b" | "--backup" => {
                if i + 1 < args.len() {
                    options.backup_dir = Some(PathBuf::from(&args[i + 1]));
                    i += 2;
                } else {
                    show_error("-b/--backup requires a directory path");
                    std::process::exit(1);
                }
            }
            arg => {
                if app_name.is_none() {
                    app_name = Some(arg.to_string());
                } else {
                    // CRITICAL FIX: Convert relative paths to absolute!
                    let arg_path = PathBuf::from(arg);
                    if arg_path.exists() {
                        // This is a file/dir path - make it absolute
                        // match arg_path.canonicalize() {
                        //     Ok(abs_path) => app_args.push(abs_path.display().to_string()),
                        //     Err(_) => app_args.push(arg.to_string()),
                        // }
                        match dunce::canonicalize(&arg_path) {
                            Ok(abs_path) => app_args.push(abs_path.display().to_string()),
                            Err(_) => app_args.push(arg.to_string()),
                        }
                    } else {
                        // Not a path, just a regular argument
                        app_args.push(arg.to_string());
                    }
                    i += 1;
                }
                i += 1;
            }
        }
    }

    let config = match load_config() {
        Ok(c) => c,
        Err(e) => {
            show_error(&format!("Failed to load config: {}", e));
            std::process::exit(1);
        }
    };

    let app_command = if let Some(app) = app_name {
        if app_args.is_empty() {
            app
        } else {
            format!("{} {}", app, app_args.join(" "))
        }
    } else if let Some(ref default) = config.default_app {
        default.clone()
    } else {
        show_error("No app specified and no default_app in config");
        std::process::exit(1);
    };

    if let Err(e) = run_in_terminal(&app_command, &config, &options) {
        show_error(&format!("Failed to run terminal: {}", e));
        std::process::exit(1);
    }
}