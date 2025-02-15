use std::sync::Arc;

use russh::client::{self, AuthResult, Handle};
use snafu::{whatever, ResultExt};
use tokio::io::AsyncReadExt;
use tracing::{info, warn};

use crate::error;

use super::{dev::*, Client, SSHSession};

#[derive(Debug)]
pub struct SSHUserConfig {
    pub hid: String,
    pub host: String,
    pub is_system: bool,
    pub os: Option<String>,
    pub passwd: Option<String>,
}

impl SSHUserConfig {
    pub fn new(hid: impl Into<String>, host: impl Into<String>) -> Self {
        Self {
            hid: hid.into(),
            host: host.into(),
            is_system: false,
            os: None,
            passwd: None,
        }
    }
}
#[async_trait::async_trait]
impl UserCast for SSHUserConfig {
    async fn cast(self) -> crate::Result<User> {
        let (h, user) = connect(self.host, self.passwd).await?;
        let mut p = detect(&h, user).await?;
        let channel = h.channel_open_session().await?;
        channel.request_subsystem(true, "sftp").await?;

        let sftp = russh_sftp::client::SftpSession::new(channel.into_stream())
            .await
            .with_context(|_| error::SFTPSnafu {
                about: "crate sftp session",
            })?;
        if let Some(os) = self.os {
            p = p.os(os);
        }
        let command_util = (&p).into();
        let sys = SSHSession {
            session: h,
            sftp,
            command_util,
        };

        User::new(self.hid, p, self.is_system, sys).await
    }
}

async fn connect(host: String, passwd: Option<String>) -> crate::Result<(Handle<Client>, String)> {
    let host_cfg = russh_config::parse_home(&host)?;
    let config = client::Config::default();
    let config = Arc::new(config);
    let sh = Client {};

    let mut session =
        client::connect(config, (host_cfg.host_name.clone(), host_cfg.port), sh).await?;

    let mut res = session.authenticate_none(&host_cfg.user).await?;
    let AuthResult::Failure {
        mut remaining_methods,
    } = res
    else {
        return Ok((session, host_cfg.user));
    };
    warn!("authenticate_none failed");
    use russh::{keys, MethodKind};
    if let (Some(path), true) = (
        host_cfg.identity_file,
        remaining_methods.contains(&MethodKind::PublicKey),
    ) {
        let kp = keys::load_secret_key(&path, None)?;
        res = session
            .authenticate_publickey(
                &host_cfg.user,
                keys::PrivateKeyWithHashAlg::new(Arc::new(kp), None),
            )
            .await?;
        let AuthResult::Failure {
            remaining_methods: s,
        } = res
        else {
            return Ok((session, host_cfg.user));
        };
        warn!("authenticate_publickey with {} failed", path);
        remaining_methods = s;
    }
    if let (Some(passwd), true) = (passwd, remaining_methods.contains(&MethodKind::Password)) {
        res = session
            .authenticate_password(&host_cfg.user, passwd)
            .await?;
        if res.success() {
            return Ok((session, host_cfg.user));
        }
        warn!("authenticate_password failed");
    }
    whatever!(
        "ssh connect {} {} {} failed",
        host,
        host_cfg.host_name,
        host_cfg.user
    );
}

async fn detect(h: &Handle<Client>, user: String) -> crate::Result<Params> {
    let mut channel = h.channel_open_session().await?;
    channel.exec(true, "env").await?;
    let mut output = String::with_capacity(1024);
    channel
        .make_reader()
        .read_to_string(&mut output)
        .await
        .with_context(|_| error::IoSnafu { about: "read env" })?;
    let mut p = Params::new(user);
    fn extract<const S: usize>(output: &str, keys: &[&str; S]) -> [Option<String>; S] {
        let mut values = [const { None }; S];
        for line in output.split('\n') {
            let mut kv = line.splitn(2, '=');
            let Some(key) = kv.next() else {
                continue;
            };
            let Some(value) = kv.next() else {
                continue;
            };
            if let Some(i) = keys.iter().position(|&k| key == k) {
                values[i] = Some(value.to_string());
            }
        }
        values
    }
    let [home, session] = extract(&output, &["HOME", "XDG_SESSION_TYPE"]);
    if let Some(home) = home {
        info!("home: {}", home);
        p = p.home(home);
    }
    if let Some(session) = session {
        info!("session: {}", session);
        p = p.session(session);
    }
    channel = h.channel_open_session().await?;
    channel
        // .exec(true, "cat /etc/os-release 2>/dev/null")
        .exec(
            true,
            "sh -c 'cat /etc/os-release 2>/dev/null || cat /usr/lib/os-release 2>/dev/null'",
        )
        .await?;
    channel
        .make_reader()
        .read_to_string(&mut output)
        .await
        .with_context(|_| error::IoSnafu {
            about: "read /etc/os-release",
        })?;
    let [os] = extract(&output, &["ID"]);
    if let Some(os) = os {
        info!("os: {}", os);
        p = p.os(os);
    }
    Ok(p)
}
