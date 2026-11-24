//! Synthetic keyboard input into whatever app has focus (copy, paste,
//! typing the "thinking" dots). Used by the global hotkey's in-place mode.
//!
//! * Linux Wayland: the XDG RemoteDesktop portal, sending keysyms directly.
//!   The session's restore token is saved, so the desktop's "Remote desktop"
//!   consent dialog appears once and is remembered.
//! * Everything else (Windows, macOS, Linux X11): `enigo`, on one dedicated
//!   OS thread (it isn't `Send` on every platform).

use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum K {
    /// Ctrl (Cmd on macOS)
    Primary,
    Shift,
    Alt,
    Super,
    Backspace,
    Char(char),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Dir {
    Press,
    Release,
    Click,
}

async fn send(keys: Vec<(K, Dir)>) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    if crate::shortcut::is_wayland() {
        return portal::send(keys).await;
    }
    native::send(keys).await
}

async fn chord(letter: char) -> Result<(), String> {
    send(vec![
        (K::Primary, Dir::Press),
        (K::Char(letter), Dir::Click),
        (K::Primary, Dir::Release),
    ])
    .await
}

/// Release modifiers that may still be logically held from the hotkey,
/// so a synthetic Ctrl+C doesn't turn into Ctrl+Shift+C.
pub async fn release_modifiers() -> Result<(), String> {
    // On Wayland the hotkey fires on release and the portal only knows about
    // keys it pressed itself, so there is nothing to release.
    #[cfg(target_os = "linux")]
    if crate::shortcut::is_wayland() {
        return Ok(());
    }
    send(vec![
        (K::Shift, Dir::Release),
        (K::Alt, Dir::Release),
        (K::Super, Dir::Release),
        (K::Primary, Dir::Release),
    ])
    .await
}

/// Ask for input permission up front (Wayland: the desktop's Remote Desktop
/// dialog), so it doesn't steal focus in the middle of the first correction.
pub async fn warm_up() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    if crate::shortcut::is_wayland() {
        return portal::send(Vec::new()).await;
    }
    Ok(())
}

pub async fn copy() -> Result<(), String> {
    chord('c').await
}

pub async fn paste() -> Result<(), String> {
    chord('v').await
}

pub async fn type_char(c: char) -> Result<(), String> {
    send(vec![(K::Char(c), Dir::Click)]).await
}

pub async fn backspace(n: usize) -> Result<(), String> {
    send(vec![(K::Backspace, Dir::Click); n]).await
}

const KEY_GAP: Duration = Duration::from_millis(8);

mod native {
    use std::sync::mpsc;
    use std::sync::OnceLock;

    use enigo::{Direction, Enigo, Key, Keyboard, Settings};

    use super::{Dir, K, KEY_GAP};

    type Msg = (Vec<(K, Dir)>, tokio::sync::oneshot::Sender<Result<(), String>>);

    static WORKER: OnceLock<mpsc::Sender<Msg>> = OnceLock::new();

    fn map(k: K) -> Key {
        match k {
            K::Primary if cfg!(target_os = "macos") => Key::Meta,
            K::Primary => Key::Control,
            K::Shift => Key::Shift,
            K::Alt => Key::Alt,
            K::Super => Key::Meta,
            K::Backspace => Key::Backspace,
            K::Char(c) => Key::Unicode(c),
        }
    }

    fn run(enigo: &mut Enigo, keys: &[(K, Dir)]) -> Result<(), String> {
        for &(k, d) in keys {
            let d = match d {
                Dir::Press => Direction::Press,
                Dir::Release => Direction::Release,
                Dir::Click => Direction::Click,
            };
            enigo.key(map(k), d).map_err(|e| e.to_string())?;
            std::thread::sleep(KEY_GAP);
        }
        Ok(())
    }

    fn worker() -> &'static mpsc::Sender<Msg> {
        WORKER.get_or_init(|| {
            let (tx, rx) = mpsc::channel::<Msg>();
            std::thread::Builder::new()
                .name("jolly-inject".into())
                .spawn(move || {
                    let mut enigo: Option<Enigo> = None;
                    for (keys, reply) in rx {
                        if enigo.is_none() {
                            match Enigo::new(&Settings::default()) {
                                Ok(e) => enigo = Some(e),
                                Err(e) => {
                                    let _ = reply
                                        .send(Err(format!("input simulation unavailable: {e}")));
                                    continue;
                                }
                            }
                        }
                        let _ = reply.send(run(enigo.as_mut().expect("set above"), &keys));
                    }
                })
                .expect("spawn inject thread");
            tx
        })
    }

    pub async fn send(keys: Vec<(K, Dir)>) -> Result<(), String> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        worker()
            .send((keys, tx))
            .map_err(|_| "inject thread gone".to_string())?;
        rx.await.map_err(|_| "inject thread gone".to_string())?
    }
}

#[cfg(target_os = "linux")]
mod portal {
    use ashpd::desktop::remote_desktop::{
        DeviceType, KeyState, NotifyKeyboardKeysymOptions, RemoteDesktop, SelectDevicesOptions,
        StartOptions,
    };
    use ashpd::desktop::{PersistMode, Session};
    use std::time::Duration;

    use futures_util::StreamExt;
    use tokio::sync::Mutex;

    use super::{Dir, K, KEY_GAP};

    struct Conn {
        proxy: RemoteDesktop,
        session: std::sync::Arc<Session<RemoteDesktop>>,
    }

    static CONN: Mutex<Option<Conn>> = Mutex::const_new(None);

    fn token_path() -> Option<std::path::PathBuf> {
        Some(
            dirs_next::data_dir()?
                .join("com.jolly.desktop")
                .join("remote-desktop-token"),
        )
    }

    async fn connect() -> Result<Conn, String> {
        let proxy = RemoteDesktop::new().await.map_err(|e| e.to_string())?;
        let session = proxy
            .create_session(Default::default())
            .await
            .map_err(|e| e.to_string())?;
        let token = token_path().and_then(|p| std::fs::read_to_string(p).ok());
        proxy
            .select_devices(
                &session,
                SelectDevicesOptions::default()
                    .set_devices(DeviceType::KEYBOARD)
                    .set_persist_mode(PersistMode::ExplicitlyRevoked)
                    .set_restore_token(token.as_deref().map(str::trim)),
            )
            .await
            .map_err(|e| e.to_string())?;
        // Shows the consent dialog unless the restore token is still valid.
        let started = proxy
            .start(&session, None, StartOptions::default())
            .await
            .map_err(|e| e.to_string())?
            .response()
            .map_err(|e| format!("remote input not allowed: {e}"))?;
        if let (Some(t), Some(p)) = (started.restore_token(), token_path()) {
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(p, t);
        }
        Ok(Conn {
            proxy,
            session: std::sync::Arc::new(session),
        })
    }

    fn keysym(k: K) -> i32 {
        match k {
            K::Primary => 0xffe3, // Control_L
            K::Shift => 0xffe1,
            K::Alt => 0xffe9,
            K::Super => 0xffeb,
            K::Backspace => 0xff08,
            // Latin-1 keysyms equal their code point; others use 0x0100_0000 + cp.
            K::Char(c) if (c as u32) < 0x100 => c as i32,
            K::Char(c) => 0x0100_0000 | c as i32,
        }
    }

    async fn notify(conn: &Conn, k: K, state: KeyState) -> Result<(), String> {
        conn.proxy
            .notify_keyboard_keysym(
                &conn.session,
                keysym(k),
                state,
                NotifyKeyboardKeysymOptions::default(),
            )
            .await
            .map_err(|e| e.to_string())
    }

    async fn run(conn: &Conn, keys: &[(K, Dir)]) -> Result<(), String> {
        for &(k, d) in keys {
            if matches!(d, Dir::Press | Dir::Click) {
                notify(conn, k, KeyState::Pressed).await?;
            }
            if matches!(d, Dir::Release | Dir::Click) {
                notify(conn, k, KeyState::Released).await?;
            }
            tokio::time::sleep(KEY_GAP).await;
        }
        Ok(())
    }

    /// A restore normally takes milliseconds; a first-time grant waits for
    /// the user to answer the desktop's dialog.
    const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

    async fn connect_logged() -> Result<Conn, String> {
        let t0 = std::time::Instant::now();
        crate::shortcut::log("remote input: connecting");
        let conn = tokio::time::timeout(CONNECT_TIMEOUT, connect())
            .await
            .map_err(|_| "timed out waiting for remote input permission".to_string())??;
        crate::shortcut::log(&format!("remote input: ready after {:?}", t0.elapsed()));
        // Log when the desktop ends the session (e.g. via the top-bar indicator).
        let session = conn.session.clone();
        tokio::spawn(async move {
            let Ok(mut closed) = session.receive_closed().await else {
                return;
            };
            if closed.next().await.is_some() {
                crate::shortcut::log("remote input: session closed by the desktop");
                let mut guard = CONN.lock().await;
                // Only forget it if it's still the current session.
                if guard
                    .as_ref()
                    .is_some_and(|c| std::sync::Arc::ptr_eq(&c.session, &session))
                {
                    *guard = None;
                }
            }
        });
        Ok(conn)
    }

    pub async fn send(keys: Vec<(K, Dir)>) -> Result<(), String> {
        let mut guard = CONN.lock().await;
        // One retry with a fresh session, in case the desktop closed the old one.
        for attempt in 0..2 {
            if guard.is_none() {
                *guard = Some(connect_logged().await?);
            }
            match run(guard.as_ref().expect("set above"), &keys).await {
                Ok(()) => return Ok(()),
                Err(e) if attempt == 0 => {
                    crate::shortcut::log(&format!("remote input failed ({e}), reconnecting"));
                    *guard = None;
                }
                Err(e) => return Err(e),
            }
        }
        unreachable!()
    }
}
