use std::{
    io::Write,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use crossterm::terminal;
use dv_api::process::{BoxedPty, BoxedPtyWriter, Interactor, WindowSize};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::Mutex,
    time::timeout,
};
use tracing::{debug, warn};

#[derive(Clone, Default)]
struct SyncHandle {
    pub pty_writer: Arc<Mutex<Option<BoxedPtyWriter>>>,
    pub ready: Arc<AtomicBool>,
}

pub struct TermInteractor {
    sync: SyncHandle,
    excl_stdout: Mutex<()>,
}

impl TermInteractor {
    pub fn new() -> std::io::Result<Self> {
        let sh = SyncHandle::default();
        tokio::spawn(sync_stdin(sh.clone()));
        // setup_stdin_nonblock()?;
        Ok(Self {
            sync: sh,
            excl_stdout: Mutex::new(()),
        })
    }
}

#[async_trait::async_trait]
impl Interactor for TermInteractor {
    async fn window_size(&self) -> WindowSize {
        let (cols, rows) = crossterm::terminal::size().expect("try to get terminal size");
        WindowSize { cols, rows }
    }
    async fn log(&self, msg: &str) {
        let g = self.excl_stdout.lock().await;
        println!("{}", msg);
        drop(g);
    }
    //TODO:more easily error handling
    async fn ask(&self, pty: BoxedPty) -> dv_api::Result<i32> {
        let (mut ctl, writer, mut reader) = pty.destruct();
        let g = self.excl_stdout.lock().await;
        terminal::enable_raw_mode()?;
        self.sync.pty_writer.lock().await.replace(writer);
        while !self.sync.ready.load(Ordering::Acquire) {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        let hsout = tokio::spawn(async move {
            let mut buf = [0; 1024];
            let mut to = std::io::stdout();
            let res: Result<(), std::io::Error> = loop {
                let n = reader.read(&mut buf).await;
                let n = match n {
                    Ok(n) => {
                        if n == 0 {
                            break Ok(());
                        }
                        n
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        continue;
                    }
                    Err(e) => break Err(e),
                };
                to.write_all(&buf[..n])?;
                to.flush()?;
            };
            res
        });
        let status = ctl.wait().await.unwrap();
        // h.abort();
        // debug!("wait for sync stdin to tx");
        let res = match timeout(Duration::from_secs(1), hsout).await {
            Ok(res) => match res {
                Ok(res) => res
                    .map(|_| 0)
                    .map_err(|e| dv_api::Error::Unknown(e.to_string())),
                Err(e) => Err(dv_api::Error::Unknown(e.to_string())),
            },
            Err(e) => Err(dv_api::Error::Unknown(e.to_string())),
        };
        terminal::disable_raw_mode()?;
        drop(g);
        self.sync.ready.store(false, Ordering::Release);
        if status == 0 {
            Ok(0)
        } else {
            let es = res
                .map(|_| status)
                .map_err(|e| dv_api::Error::Unknown(e.to_string()))?;
            Ok(es)
        }
    }
}

async fn sync_stdin(sync_handle: SyncHandle) {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    loop {
        let mut writer = sync_handle.pty_writer.lock().await;
        let Some(writer) = writer.as_mut() else {
            drop(writer);
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        };
        debug!("receive stdin ready");
        sync_handle.ready.store(true, Ordering::Release);
        debug!("start to sync stdin to pty");

        let mut key_buf = [0u8; 4];
        while sync_handle.ready.load(Ordering::Acquire) {
            if !crossterm::event::poll(Duration::from_millis(100)).expect("try to poll") {
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }
            let ev = crossterm::event::read().expect("try to read event");

            if let Event::Key(KeyEvent {
                code, modifiers, ..
            }) = ev
            {
                let bytes: &[u8] = match (modifiers, code) {
                    (KeyModifiers::CONTROL, KeyCode::Char('c')) => "\x03".as_bytes(),
                    (KeyModifiers::CONTROL, KeyCode::Left) => "\x1b[D".as_bytes(),
                    (KeyModifiers::CONTROL, KeyCode::Right) => "\x1b[C".as_bytes(),
                    (KeyModifiers::CONTROL, KeyCode::Up) => "\x1b[A".as_bytes(),
                    (KeyModifiers::CONTROL, KeyCode::Down) => "\x1b[B".as_bytes(),
                    (KeyModifiers::CONTROL, KeyCode::Char('d')) => "\x04".as_bytes(),
                    (_, KeyCode::Char(c)) => {
                        key_buf[0] = c as u8;
                        &key_buf[..1]
                    }
                    (_, KeyCode::Backspace) => "\x7f".as_bytes(),
                    (_, KeyCode::Enter) => "\r".as_bytes(),
                    (_, KeyCode::Esc) => "\x1b".as_bytes(),
                    _ => continue, //TODO:handle other keys
                };
                if let Err(e) = writer.write_all(bytes).await {
                    warn!("write to pty failed: {}", e);
                    break;
                }
                if let Err(e) = writer.flush().await {
                    warn!("flush pty failed: {}", e);
                    break;
                }
            }
        }
    }
}
