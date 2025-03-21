use std::{collections::HashMap, io::Write, time::Duration};

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use dv_api::{
    Result,
    process::{BoxedPty, BoxedPtyReader, BoxedPtyWriter, Interactor, WindowSize},
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{
        mpsc::{self, Receiver},
        oneshot,
    },
};
use tracing::{debug, warn};

pub struct TermInteractor {
    q: mpsc::Sender<Request>,
}

impl TermInteractor {
    pub fn new() -> std::io::Result<Self> {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        tokio::spawn(Ui { channel: rx }.run());
        Ok(Self { q: tx })
    }
}

#[async_trait::async_trait]
impl Interactor for TermInteractor {
    async fn window_size(&self) -> WindowSize {
        let (cols, rows) = crossterm::terminal::size().expect("try to get terminal size");
        WindowSize { cols, rows }
    }
    async fn log(&self, msg: String) {
        self.q
            .send(Request::Log(msg))
            .await
            .expect("send log request");
    }
    async fn ask(&self, pty: BoxedPty) -> dv_api::Result<i32> {
        let (mut ctl, writer, reader) = pty.destruct();
        let (tx, rx) = oneshot::channel();
        self.q
            .send(Request::Ask(Ask {
                writer,
                reader,
                exit: rx,
            }))
            .await
            .expect("send ask request");
        let ec = ctl.wait().await?;
        tx.send(()).expect("send exit signal");
        Ok(ec)
    }
    async fn confirm(&self, msg: String, opts: &[&str]) -> Result<usize> {
        let opts = opts
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let (c, s) = s
                    .split_once('/')
                    .and_then(|(c, s)| c.chars().next().map(|c| (c, s)))
                    .unwrap_or((char::from_digit(i as u32 + 1, 10).unwrap(), s));
                (c, s.to_string())
            })
            .collect();
        let (tx, rx) = oneshot::channel();
        self.q
            .send(Request::Confirm(Confirm {
                msg: msg.to_string(),
                opts,
                sel: tx,
            }))
            .await
            .expect("send confirm request");
        let sel = rx
            .await
            .map_err(|_| dv_api::Error::unknown("confirm receiver dropped"))?;
        Ok(sel)
    }
}

enum Request {
    Ask(Ask),
    Log(String),
    Confirm(Confirm),
}

struct Ui {
    channel: Receiver<Request>,
}

impl Ui {
    async fn run(mut self) {
        loop {
            match self.channel.recv().await {
                Some(Request::Ask(a)) => {
                    if let Err(e) = a.exec().await {
                        warn!("sync stdin failed: {}", e);
                    }
                }
                Some(Request::Log(msg)) => {
                    println!("{}", msg);
                }
                Some(Request::Confirm(c)) => {
                    if let Err(e) = c.exec().await {
                        warn!("confirm failed: {}", e);
                    }
                }
                Option::None => {
                    break;
                }
            }
        }
    }
}

struct Ask {
    writer: BoxedPtyWriter,
    reader: BoxedPtyReader,
    exit: oneshot::Receiver<()>,
}

impl Ask {
    async fn exec(self) -> Result<()> {
        let Ask {
            mut writer,
            mut reader,
            mut exit,
        } = self;
        enable_raw_mode()?;
        debug!("start to sync stdin to pty");
        let h = tokio::spawn(async move {
            let mut buf = [0; 1024];
            let mut to = std::io::stdout();
            loop {
                let n = reader.read(&mut buf).await?;
                if n == 0 {
                    break;
                }
                debug!("read {} bytes from pty", n);
                to.write_all(&buf[..n])?;
                to.flush()?;
            }
            Ok::<_, std::io::Error>(())
        });
        tokio::time::sleep(Duration::from_millis(100)).await;

        let mut key_buf = [0u8; 4];
        while let Err(oneshot::error::TryRecvError::Empty) = exit.try_recv() {
            if !event::poll(Duration::from_millis(100))? {
                continue;
            }
            let ev = event::read()?;
            if let Event::Key(KeyEvent {
                code, modifiers, ..
            }) = ev
            {
                let bytes: &[u8] = match (modifiers, code) {
                    (KeyModifiers::CONTROL, KeyCode::Char('c')) => "\x03".as_bytes(),
                    (KeyModifiers::CONTROL, KeyCode::Char('d')) => "\x04".as_bytes(),
                    (_, KeyCode::Left) => "\x1b[D".as_bytes(),
                    (_, KeyCode::Right) => "\x1b[C".as_bytes(),
                    (_, KeyCode::Up) => "\x1b[A".as_bytes(),
                    (_, KeyCode::Down) => "\x1b[B".as_bytes(),
                    (_, KeyCode::Char(c)) => {
                        key_buf[0] = c as u8;
                        &key_buf[..1]
                    }
                    (_, KeyCode::Backspace) => "\x7f".as_bytes(),
                    (_, KeyCode::Enter) => "\r".as_bytes(),
                    (_, KeyCode::Esc) => "\x1b".as_bytes(),
                    _ => continue, //TODO:handle other keys
                };
                writer.write_all(bytes).await?;
                writer.flush().await?;
            } else if let Event::Resize(cols, rows) = ev {
                writer.window_change(cols, rows).await?;
            }
        }
        h.abort();
        disable_raw_mode()?;
        Ok(())
    }
}

struct Confirm {
    msg: String,
    opts: Vec<(char, String)>,
    sel: oneshot::Sender<usize>,
}

impl Confirm {
    async fn exec(self) -> Result<()> {
        println!("{}", self.msg);
        print!("opts [");
        for opt in &self.opts {
            print!("{}: {}, ", opt.0, opt.1);
        }
        print!("]:");
        std::io::stdout().flush()?;
        let mut hash = self
            .opts
            .iter()
            .enumerate()
            .map(|(i, (c, _))| (*c, i))
            .collect::<HashMap<_, _>>();
        hash.reserve(0);
        loop {
            if !event::poll(Duration::from_millis(100))? {
                continue;
            }
            let ev = event::read()?;
            if let Event::Key(KeyEvent {
                code,
                modifiers: KeyModifiers::NONE,
                ..
            }) = ev
            {
                let KeyCode::Char(c) = code else {
                    continue;
                };
                if let Some(&i) = hash.get(&c) {
                    self.sel.send(i).expect("send confirm selection");
                    return Ok(());
                }
            }
        }
    }
}
