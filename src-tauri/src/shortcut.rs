//! Global "fix it" hotkey.
//!
//! * Windows / macOS / Linux X11: `tauri-plugin-global-shortcut`.
//! * Linux Wayland: the XDG GlobalShortcuts portal (KDE, GNOME 48+, Hyprland),
//!   falling back to the plugin (works for XWayland-focused apps).
//! * Everywhere: `jolly --fix` (via single-instance) as a bindable command.
//!
//! When triggered, highlighted text in the focused app is replaced in place
//! (with an animated "..." placeholder while the model works); with nothing
//! highlighted the clipboard is corrected instead.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tauri_plugin_store::StoreExt;

use crate::inject;

pub const DEFAULT_SHORTCUT: &str = "CommandOrControl+Shift+J";

static BUSY: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutStatus {
    /// The configured accelerator, `None` when disabled.
    pub accelerator: Option<String>,
    pub active: bool,
    /// "native", "portal" or "none".
    pub backend: &'static str,
    /// What the desktop actually bound (portal may differ from our request).
    pub trigger_description: Option<String>,
    pub error: Option<String>,
}

#[derive(Default)]
pub struct ShortcutRuntime {
    status: ShortcutStatus,
    #[cfg(target_os = "linux")]
    portal_task: Option<tauri::async_runtime::JoinHandle<()>>,
}

pub type ShortcutManager = Mutex<ShortcutRuntime>;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Finished {
    ok: bool,
    /// "inline" (selection replaced) or "clipboard".
    mode: &'static str,
    original: Option<String>,
    text: Option<String>,
    code: Option<String>,
}

pub fn is_wayland() -> bool {
    cfg!(target_os = "linux")
        && (std::env::var_os("WAYLAND_DISPLAY").is_some()
            || std::env::var("XDG_SESSION_TYPE").is_ok_and(|v| v.eq_ignore_ascii_case("wayland")))
}

/// Everyday editing shortcuts (copy, paste, cut, undo, select all, …) that
/// must never be grabbed system-wide.
pub fn is_reserved(accel: &str) -> bool {
    let parts: Vec<String> = accel.split('+').map(|p| p.to_ascii_lowercase()).collect();
    let Some((key, mods)) = parts.split_last() else {
        return false;
    };
    let key = key.strip_prefix("key").unwrap_or(key);
    let primary = |m: &String| {
        matches!(
            m.as_str(),
            "commandorcontrol" | "cmdorctrl" | "control" | "ctrl" | "command" | "cmd" | "super"
        )
    };
    mods.iter().any(primary)
        && mods.iter().all(|m| primary(m) || m == "shift")
        && matches!(key, "a" | "c" | "v" | "x" | "z" | "y")
}

/// Read the saved accelerator. Missing key → default; explicit null → disabled.
/// A reserved combination (saved by an older build) falls back to the default.
pub fn saved_accelerator(app: &AppHandle) -> Option<String> {
    match app
        .store("settings.json")
        .ok()
        .and_then(|s| s.get("globalShortcut"))
    {
        None => Some(DEFAULT_SHORTCUT.to_string()),
        Some(v) => v.as_str().filter(|s| !s.is_empty()).map(|s| {
            if is_reserved(s) {
                DEFAULT_SHORTCUT.to_string()
            } else {
                s.to_string()
            }
        }),
    }
}

pub fn inline_enabled(app: &AppHandle) -> bool {
    app.store("settings.json")
        .ok()
        .and_then(|s| s.get("inlineReplace"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true)
}

/// Convert a Tauri accelerator ("CommandOrControl+Shift+J") into the XDG
/// shortcut trigger format ("CTRL+SHIFT+j").
pub fn to_portal_trigger(accel: &str) -> String {
    accel
        .split('+')
        .map(|part| match part.to_ascii_lowercase().as_str() {
            "commandorcontrol" | "cmdorctrl" | "control" | "ctrl" => "CTRL".to_string(),
            "shift" => "SHIFT".to_string(),
            "alt" | "option" => "ALT".to_string(),
            "super" | "meta" | "command" | "cmd" => "LOGO".to_string(),
            key => {
                let k = key.strip_prefix("key").unwrap_or(key);
                let k = k.strip_prefix("digit").unwrap_or(k);
                if k.len() == 1 {
                    k.to_string()
                } else {
                    // Named keys (F5, Space, …) keep XKB-style capitalisation.
                    let mut c = k.chars();
                    c.next()
                        .map(|f| f.to_ascii_uppercase().to_string() + c.as_str())
                        .unwrap_or_default()
                }
            }
        })
        .collect::<Vec<_>>()
        .join("+")
}

/// (Re-)register the hotkey. `None` disables it.
pub async fn apply(app: &AppHandle, accel: Option<String>) -> ShortcutStatus {
    // Tear down whatever was registered before.
    let _ = app.global_shortcut().unregister_all();
    #[cfg(target_os = "linux")]
    {
        let task = app
            .state::<ShortcutManager>()
            .lock()
            .unwrap()
            .portal_task
            .take();
        if let Some(task) = task {
            task.abort();
            portal::clear().await;
        }
    }

    let mut status = ShortcutStatus {
        accelerator: accel.clone(),
        backend: "none",
        ..Default::default()
    };

    if let Some(accel) = accel {
        // Only the Linux portal path assigns this, so the type must be explicit.
        #[cfg_attr(not(target_os = "linux"), allow(unused_mut))]
        let mut portal_error: Option<String> = None;

        #[cfg(target_os = "linux")]
        if is_wayland() {
            match portal::bind(app, &accel).await {
                Ok((task, desc)) => {
                    app.state::<ShortcutManager>().lock().unwrap().portal_task = Some(task);
                    status.active = true;
                    status.backend = "portal";
                    status.trigger_description = desc;
                }
                Err(e) => {
                    eprintln!("[jolly] GlobalShortcuts portal unavailable: {e}");
                    portal_error = Some(e);
                }
            }
        }

        if !status.active {
            match register_native(app, &accel) {
                Ok(()) => {
                    status.active = true;
                    status.backend = "native";
                    // On Wayland the native path only sees XWayland windows.
                    status.error = portal_error.map(|e| {
                        format!("Desktop portal unavailable ({e}); hotkey only works in X11 apps")
                    });
                }
                Err(e) => {
                    status.error = Some(match portal_error {
                        Some(p) => format!("{e} (portal: {p})"),
                        None => e,
                    })
                }
            }
        }
    }

    app.state::<ShortcutManager>().lock().unwrap().status = status.clone();
    status
}

fn register_native(app: &AppHandle, accel: &str) -> Result<(), String> {
    let shortcut: Shortcut = accel
        .parse()
        .map_err(|e| format!("Invalid shortcut '{accel}': {e}"))?;
    app.global_shortcut()
        .on_shortcut(shortcut, |app, _shortcut, event| {
            // Fire on release so the user's modifier keys are up before we
            // send synthetic Ctrl+C / Ctrl+V.
            if event.state == ShortcutState::Released {
                trigger(app.clone());
            }
        })
        .map_err(|e| format!("Couldn't register '{accel}': {e}"))
}

/// Open the desktop's shortcut settings dialog (Wayland portal only).
pub async fn configure_system() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    return portal::configure().await;
    #[cfg(not(target_os = "linux"))]
    Err("only available on Linux Wayland".into())
}

pub fn status(app: &AppHandle) -> ShortcutStatus {
    app.state::<ShortcutManager>()
        .lock()
        .unwrap()
        .status
        .clone()
}

/// Run one correction in the background. Ignored while one is running.
pub fn trigger(app: AppHandle) {
    if BUSY.swap(true, Ordering::SeqCst) {
        return;
    }
    tauri::async_runtime::spawn(async move {
        let _ = app.emit("correction-started", ());
        let finished = run(&app).await;
        let _ = app.emit("correction-finished", finished);
        BUSY.store(false, Ordering::SeqCst);
    });
}

/// Timestamped log line (UTC), to line hotkey events up with desktop dialogs.
pub fn log(msg: &str) {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or_default();
    let day = secs % 86_400.0;
    let (h, m, s) = ((day / 3600.0) as u32, (day % 3600.0 / 60.0) as u32, day % 60.0);
    eprintln!("[jolly {h:02}:{m:02}:{s:06.3}Z] {msg}");
}

async fn run(app: &AppHandle) -> Finished {
    log("hotkey: triggered");
    if inline_enabled(app) {
        // Never let a stuck desktop dialog block the hotkey for good.
        match tokio::time::timeout(Duration::from_secs(40), grab_selection(app)).await {
            Ok(Ok(Some(original))) => {
                log(&format!("hotkey: selection {} chars, replacing in place", original.chars().count()));
                let done = replace_inline(app, original).await;
                log(&format!("hotkey: done (ok={})", done.ok));
                return done;
            }
            Ok(Ok(None)) => log("hotkey: nothing selected, fixing clipboard"),
            Ok(Err(e)) => log(&format!("hotkey: in-place unavailable ({e}), fixing clipboard")),
            Err(_) => log("hotkey: selecting text timed out, fixing clipboard"),
        }
    }
    let done = fix_clipboard(app).await;
    log(&format!("hotkey: done (ok={}, code={:?})", done.ok, done.code));
    done
}

/// Copy the focused app's selection. `Ok(None)` when nothing is selected.
/// The user's clipboard is restored in that case.
async fn grab_selection(app: &AppHandle) -> Result<Option<String>, String> {
    let clip = app.clipboard();
    let previous = clip.read_text().ok();
    let sentinel = format!("\u{2063}jolly-{}\u{2063}", uuid::Uuid::new_v4());
    clip.write_text(sentinel.clone())
        .map_err(|e| e.to_string())?;

    // Let the user's fingers leave the hotkey: a still-held Super/Shift would
    // turn our Ctrl+C into Super+Ctrl+C. On Wayland we can't query or release
    // physical modifiers, and the portal fires when the *key* (J) comes up,
    // usually a moment before the modifier, so wait longer there.
    let settle = if is_wayland() { 350 } else { 60 };
    tokio::time::sleep(Duration::from_millis(settle)).await;
    let copied = async {
        inject::release_modifiers().await?;
        inject::copy().await?;
        // Clipboard changes can take a while to cross the X11/Wayland bridge.
        let mut last = String::new();
        for _ in 0..40 {
            tokio::time::sleep(Duration::from_millis(30)).await;
            match clip.read_text() {
                Ok(t) if t != sentinel => return Ok(Some(t)),
                Ok(_) => last = "unchanged".into(),
                Err(e) => last = format!("read error: {e}"),
            }
        }
        log(&format!("hotkey: clipboard after Ctrl+C: {last}"));
        Ok(None)
    }
    .await;

    match copied {
        Ok(Some(text)) if !text.trim().is_empty() => Ok(Some(text)),
        other => {
            if let Some(prev) = previous {
                let _ = clip.write_text(prev);
            }
            other.map(|_| None)
        }
    }
}

/// Replace the (still highlighted) selection with an animated placeholder,
/// correct it, then paste the result over the placeholder.
async fn replace_inline(app: &AppHandle, original: String) -> Finished {
    // Typing the first dot replaces the selection.
    let mut typed = 0usize;
    let animation_ok = inject::type_char('.').await.is_ok();
    if animation_ok {
        typed = 1;
    }

    let correction = crate::commands::run_correction(app, &original);
    tokio::pin!(correction);
    // Dot count cycles 1 → 2 → 3 → 2 → 1 …
    let mut growing = true;
    let mut ticker = tokio::time::interval(Duration::from_millis(300));
    ticker.tick().await;
    let result = loop {
        tokio::select! {
            r = &mut correction => break r,
            _ = ticker.tick(), if animation_ok => {
                if growing && typed < 3 {
                    if inject::type_char('.').await.is_ok() { typed += 1; }
                } else if typed > 1 && inject::backspace(1).await.is_ok() {
                    typed -= 1;
                }
                if typed >= 3 { growing = false; }
                if typed <= 1 { growing = true; }
            }
        }
    };

    let _ = inject::backspace(typed).await;
    let clip = app.clipboard();

    let (paste_text, finished) = match result {
        Ok(text) if !text.trim().is_empty() => (
            text.clone(),
            Finished {
                ok: true,
                mode: "inline",
                original: Some(original.clone()),
                text: Some(text),
                code: None,
            },
        ),
        Ok(_) => (
            original.clone(),
            failed("inline", &original, "empty_result"),
        ),
        Err(code) => (original.clone(), failed("inline", &original, &code)),
    };

    // Paste the result (or the original back on failure), then leave the
    // original in the clipboard so Ctrl+V restores it.
    if clip.write_text(paste_text).is_ok() {
        let _ = inject::paste().await;
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    let _ = clip.write_text(original.as_str());
    finished
}

fn failed(mode: &'static str, original: &str, code: &str) -> Finished {
    Finished {
        ok: false,
        mode,
        original: Some(original.to_string()),
        text: None,
        code: Some(code.to_string()),
    }
}

async fn fix_clipboard(app: &AppHandle) -> Finished {
    let clip = app.clipboard();
    let original = match clip.read_text() {
        Ok(t) => t,
        Err(_) => {
            return Finished {
                ok: false,
                mode: "clipboard",
                original: None,
                text: None,
                code: Some("clipboard_unreadable".into()),
            }
        }
    };
    if original.trim().is_empty() {
        return failed("clipboard", &original, "nothing_to_fix");
    }
    match crate::commands::run_correction(app, &original).await {
        Ok(text) if !text.trim().is_empty() => {
            if clip.write_text(text.clone()).is_err() {
                return failed("clipboard", &original, "clipboard_unwritable");
            }
            Finished {
                ok: true,
                mode: "clipboard",
                original: Some(original),
                text: Some(text),
                code: None,
            }
        }
        Ok(_) => failed("clipboard", &original, "empty_result"),
        Err(code) => failed("clipboard", &original, &code),
    }
}

#[cfg(target_os = "linux")]
mod portal {
    use std::sync::Arc;

    use ashpd::desktop::global_shortcuts::{
        BindShortcutsOptions, ConfigureShortcutsOptions, GlobalShortcuts, NewShortcut, Shortcut,
    };
    use ashpd::desktop::Session;
    use futures_util::StreamExt;
    use tauri::{AppHandle, Emitter, Manager};

    const SHORTCUT_ID: &str = "fix";

    type Active = (Arc<GlobalShortcuts>, Arc<Session<GlobalShortcuts>>);
    static ACTIVE: tokio::sync::Mutex<Option<Active>> = tokio::sync::Mutex::const_new(None);

    fn describe(shortcuts: &[Shortcut]) -> Option<String> {
        shortcuts
            .iter()
            .find(|s| s.id() == SHORTCUT_ID)
            .map(|s| s.trigger_description().to_string())
            .filter(|d| !d.is_empty())
    }

    pub async fn clear() {
        if let Some((_, session)) = ACTIVE.lock().await.take() {
            let _ = session.close().await;
        }
    }

    /// Open the desktop's own dialog for changing the shortcut. Changes come
    /// back through the `ShortcutsChanged` signal handled in `bind`.
    pub async fn configure() -> Result<(), String> {
        let guard = ACTIVE.lock().await;
        let (proxy, session) = guard
            .as_ref()
            .ok_or("shortcut not bound via the desktop portal")?;
        proxy
            .configure_shortcuts(session, None, ConfigureShortcutsOptions::default())
            .await
            .map_err(|e| e.to_string())
    }

    /// Write a hidden `~/.local/share/applications/<id>.desktop` so the portal
    /// can resolve our app id. Packages install `Jolly.desktop`, which the
    /// portal doesn't associate with `com.jolly.desktop`.
    fn ensure_desktop_file(app_id: &str) -> Result<(), String> {
        let dir = dirs_next::data_dir()
            .ok_or("no data dir")?
            .join("applications");
        // AppImages run from a temporary mount; $APPIMAGE is the stable path.
        let exe = std::env::var_os("APPIMAGE")
            .map(std::path::PathBuf::from)
            .or_else(|| std::env::current_exe().ok())
            .ok_or("unknown executable path")?;
        let contents = format!(
            "[Desktop Entry]\nType=Application\nName=Jolly\nExec=\"{}\" %U\nIcon=jolly\nNoDisplay=true\n",
            exe.display()
        );
        let path = dir.join(format!("{app_id}.desktop"));
        if std::fs::read_to_string(&path).ok().as_deref() == Some(contents.as_str()) {
            return Ok(());
        }
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        std::fs::write(&path, contents).map_err(|e| e.to_string())
    }

    /// Bind via the GlobalShortcuts portal. Returns the listener task and the
    /// trigger the desktop actually assigned.
    pub async fn bind(
        app: &AppHandle,
        accel: &str,
    ) -> Result<(tauri::async_runtime::JoinHandle<()>, Option<String>), String> {
        // Unsandboxed apps must tell the portal who they are. GNOME only
        // accepts an app id that has a matching `<id>.desktop` file.
        let app_id = app.config().identifier.clone();
        if let Err(e) = ensure_desktop_file(&app_id) {
            eprintln!("[jolly] Couldn't write portal desktop file: {e}");
        }
        if let Ok(id) = ashpd::AppID::try_from(app_id.as_str()) {
            // Fails harmlessly with "already associated" on re-registration.
            if let Err(e) = ashpd::register_host_app(id).await {
                if !e.to_string().contains("already associated") {
                    eprintln!("[jolly] Portal host-app registration failed: {e}");
                }
            }
        }

        let proxy = GlobalShortcuts::new().await.map_err(|e| e.to_string())?;
        let session = proxy
            .create_session(Default::default())
            .await
            .map_err(|e| e.to_string())?;
        let trigger = super::to_portal_trigger(accel);
        let shortcut = NewShortcut::new(SHORTCUT_ID, "Fix spelling (selection or clipboard)")
            .preferred_trigger(trigger.as_str());
        let bound = proxy
            .bind_shortcuts(&session, &[shortcut], None, BindShortcutsOptions::default())
            .await
            .map_err(|e| e.to_string())?
            .response()
            .map_err(|e| e.to_string())?;
        let desc = describe(bound.shortcuts());

        // Fire on release (Deactivated) so modifiers are up before injecting.
        let mut released = proxy
            .receive_deactivated()
            .await
            .map_err(|e| e.to_string())?;
        let mut changed = proxy
            .receive_shortcuts_changed()
            .await
            .map_err(|e| e.to_string())?;
        *ACTIVE.lock().await = Some((Arc::new(proxy), Arc::new(session)));

        let app = app.clone();
        let task = tauri::async_runtime::spawn(async move {
            loop {
                tokio::select! {
                    Some(ev) = released.next() => {
                        if ev.shortcut_id() == SHORTCUT_ID {
                            super::trigger(app.clone());
                        }
                    }
                    Some(ev) = changed.next() => {
                        // The user rebound it in the desktop's settings.
                        let status = {
                            let state = app.state::<super::ShortcutManager>();
                            let mut state = state.lock().unwrap();
                            state.status.trigger_description = describe(ev.shortcuts());
                            state.status.clone()
                        };
                        let _ = app.emit("shortcut-status", status);
                    }
                    else => break,
                }
            }
        });
        Ok((task, desc))
    }
}

#[cfg(test)]
mod tests {
    use super::to_portal_trigger;

    #[test]
    fn editing_shortcuts_are_reserved() {
        use super::is_reserved;
        assert!(is_reserved("CommandOrControl+C"));
        assert!(is_reserved("Ctrl+Shift+KeyZ"));
        assert!(!is_reserved("CommandOrControl+Shift+J"));
        assert!(!is_reserved("Alt+C"));
    }

    #[test]
    fn portal_trigger_format() {
        assert_eq!(
            to_portal_trigger("CommandOrControl+Shift+J"),
            "CTRL+SHIFT+j"
        );
        assert_eq!(to_portal_trigger("Alt+Super+KeyK"), "ALT+LOGO+k");
        assert_eq!(to_portal_trigger("Ctrl+Digit1"), "CTRL+1");
        assert_eq!(to_portal_trigger("Shift+F5"), "SHIFT+F5");
    }
}
