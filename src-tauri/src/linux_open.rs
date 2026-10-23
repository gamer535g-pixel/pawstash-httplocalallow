//! Children of an AppImage inherit its GTK/GIO/GStreamer search paths, load its Ubuntu 22.04
//! modules and die on newer distros, so the host environment is restored before xdg-open.

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
const APPIMAGE_ONLY_VARS: &[&str] = &[
    "APPDIR",
    "APPIMAGE",
    "ARGV0",
    "OWD",
    "GTK_THEME",
    "GST_REGISTRY_REUSE_PLUGIN_SCANNER",
];

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
const APPDIR_PATH_VARS: &[&str] = &[
    "PATH",
    "LD_LIBRARY_PATH",
    "LD_PRELOAD",
    "XDG_DATA_DIRS",
    "GIO_EXTRA_MODULES",
    "GIO_MODULE_DIR",
    "GDK_PIXBUF_MODULE_FILE",
    "GDK_PIXBUF_MODULEDIR",
    "GTK_PATH",
    "GTK_EXE_PREFIX",
    "GTK_DATA_PREFIX",
    "GTK_IM_MODULE_FILE",
    "GSETTINGS_SCHEMA_DIR",
    "GI_TYPELIB_PATH",
    "GST_PLUGIN_PATH",
    "GST_PLUGIN_PATH_1_0",
    "GST_PLUGIN_SYSTEM_PATH",
    "GST_PLUGIN_SYSTEM_PATH_1_0",
    "GST_PLUGIN_SYSTEM_PATH_0_10",
    "GST_PLUGIN_SCANNER",
    "GST_PLUGIN_SCANNER_1_0",
    "GST_PLUGIN_SCANNER_0_10",
    "GST_PTP_HELPER",
    "GST_PTP_HELPER_1_0",
    "GST_PTP_HELPER_0_10",
    "PYTHONHOME",
    "PYTHONPATH",
    "PERLLIB",
    "QT_PLUGIN_PATH",
];

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
const FALLBACK_PATH: &str = "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin";

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn strip_appdir_entries(value: &str, appdir: &str) -> Option<String> {
    let appdir = appdir.trim_end_matches('/');
    let kept: Vec<&str> = value
        .split(':')
        .filter(|entry| {
            !entry.is_empty()
                && (appdir.is_empty()
                    || (*entry != appdir
                        && !entry
                            .strip_prefix(appdir)
                            .is_some_and(|rest| rest.starts_with('/'))))
        })
        .collect();
    (!kept.is_empty()).then(|| kept.join(":"))
}

#[cfg(target_os = "linux")]
mod host {
    use super::{strip_appdir_entries, APPDIR_PATH_VARS, APPIMAGE_ONLY_VARS, FALLBACK_PATH};
    use std::ffi::{OsStr, OsString};
    use std::io::Read;
    use std::process::{Command, Stdio};
    use std::sync::{mpsc, Arc, Mutex};
    use std::time::{Duration, Instant};

    /// Only xdg-open's generic fallback blocks; past this the handler has started.
    const EXIT_WAIT: Duration = Duration::from_secs(3);
    const POLL_INTERVAL: Duration = Duration::from_millis(40);
    /// The handler can inherit stderr and hold it open after xdg-open exits.
    const STDERR_GRACE: Duration = Duration::from_millis(250);
    const STDERR_CAPTURE_LIMIT: usize = 4096;

    pub fn apply_host_env(cmd: &mut Command) {
        let appdir = std::env::var("APPDIR").ok();
        if appdir.is_none() && std::env::var_os("APPIMAGE").is_none() {
            return;
        }
        for name in APPIMAGE_ONLY_VARS {
            cmd.env_remove(name);
        }
        let Some(appdir) = appdir else {
            return;
        };
        for name in APPDIR_PATH_VARS {
            let Some(raw) = std::env::var_os(name) else {
                continue;
            };
            let Some(value) = raw.to_str() else {
                continue;
            };
            match strip_appdir_entries(value, &appdir) {
                Some(kept) if kept == value => {}
                Some(kept) => {
                    cmd.env(name, kept);
                }
                None if *name == "PATH" => {
                    cmd.env(name, FALLBACK_PATH);
                }
                None => {
                    cmd.env_remove(name);
                }
            }
        }
    }

    pub async fn open(target: impl Into<OsString>) -> Result<(), String> {
        let target = target.into();
        tauri::async_runtime::spawn_blocking(move || open_blocking(&target))
            .await
            .map_err(|error| format!("xdg-open task failed: {error}"))?
    }

    fn open_blocking(target: &OsStr) -> Result<(), String> {
        let program = if std::path::Path::new("/usr/bin/xdg-open").is_file() {
            "/usr/bin/xdg-open"
        } else {
            "xdg-open"
        };
        let mut cmd = Command::new(program);
        apply_host_env(&mut cmd);
        let mut child = cmd
            .arg(target)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("Failed to run xdg-open: {error}"))?;

        let captured = Arc::new(Mutex::new(Vec::new()));
        let (eof_tx, eof_rx) = mpsc::channel::<()>();
        if let Some(mut stderr) = child.stderr.take() {
            let captured = captured.clone();
            std::thread::spawn(move || {
                let mut chunk = [0u8; 1024];
                loop {
                    match stderr.read(&mut chunk) {
                        Ok(0) => break,
                        Ok(read) => {
                            if let Ok(mut buffer) = captured.lock() {
                                let room = STDERR_CAPTURE_LIMIT.saturating_sub(buffer.len());
                                buffer.extend_from_slice(&chunk[..read.min(room)]);
                            }
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                        Err(error) => {
                            tracing::debug!("Stopped reading xdg-open stderr: {error}");
                            break;
                        }
                    }
                }
                let _ = eof_tx.send(());
            });
        }

        let deadline = Instant::now() + EXIT_WAIT;
        loop {
            match child.try_wait() {
                Ok(Some(status)) if status.success() => return Ok(()),
                Ok(Some(status)) => {
                    if eof_rx.recv_timeout(STDERR_GRACE).is_err() {
                        tracing::debug!("xdg-open stderr is still held open by a child process");
                    }
                    let stderr = captured
                        .lock()
                        .map(|buffer| String::from_utf8_lossy(&buffer).trim().to_string())
                        .unwrap_or_default();
                    let target = target.to_string_lossy();
                    tracing::warn!("xdg-open {target:?} failed with {status}: {stderr}");
                    return Err(if stderr.is_empty() {
                        format!("xdg-open failed ({status})")
                    } else {
                        format!("xdg-open failed ({status}): {stderr}")
                    });
                }
                Ok(None) if Instant::now() >= deadline => {
                    let target = target.to_string_lossy().into_owned();
                    std::thread::spawn(move || match child.wait() {
                        Ok(status) if !status.success() => {
                            tracing::warn!("xdg-open {target:?} exited late with {status}");
                        }
                        Ok(_) => {}
                        Err(error) => {
                            tracing::warn!("Failed to reap xdg-open for {target:?}: {error}");
                        }
                    });
                    return Ok(());
                }
                Ok(None) => std::thread::sleep(POLL_INTERVAL),
                Err(error) => return Err(format!("Failed to wait for xdg-open: {error}")),
            }
        }
    }
}

#[cfg(target_os = "linux")]
pub use host::{apply_host_env, open};

#[cfg(test)]
mod tests {
    use super::strip_appdir_entries;

    const APPDIR: &str = "/tmp/.mount_PawstaAbC123";

    #[test]
    fn strips_appdir_entries_and_keeps_host_ones() {
        let appdir_slash = format!("{APPDIR}/");
        let sibling = format!("{APPDIR}-other/lib");
        for (value, appdir, expected) in [
            (
                format!("{APPDIR}/usr/bin/:{APPDIR}/usr/sbin/:{APPDIR}/usr/games/:{APPDIR}/bin/:{APPDIR}/sbin/:/usr/local/bin:/usr/bin"),
                APPDIR,
                Some("/usr/local/bin:/usr/bin"),
            ),
            (
                format!("{APPDIR}/usr/share/:{APPDIR}/usr/share:/usr/share:/usr/local/share:"),
                APPDIR,
                Some("/usr/share:/usr/local/share"),
            ),
            (format!("{APPDIR}//usr/share/glib-2.0/schemas"), APPDIR, None),
            (APPDIR.to_string(), APPDIR, None),
            (String::new(), APPDIR, None),
            (format!("{sibling}:{APPDIR}/usr/lib"), &appdir_slash, Some(sibling.as_str())),
            ("/usr/bin::/bin".to_string(), "", Some("/usr/bin:/bin")),
        ] {
            assert_eq!(strip_appdir_entries(&value, appdir).as_deref(), expected, "{value}");
        }
    }
}
