use std::sync::Arc;

use russh::client::{self, AuthResult, Handle};
use snafu::{whatever, ResultExt};
use tracing::warn;

use crate::error;

use super::{dev::*, Client, SSHSession};

#[derive(Debug)]
pub struct SSHUserConfig {
    pub uid: String,
    pub hid: String,
    pub host: String,
    pub is_system: bool,
    pub os: Option<String>,
    pub passwd: Option<String>,
}

impl SSHUserConfig {
    pub fn new(uid: impl Into<String>, hid: impl Into<String>, host: impl Into<String>) -> Self {
        Self {
            uid: uid.into(),
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
        let id = self.uid.clone();
        let (h, user) = connect(self.host, self.passwd).await?;
        let mut env = detect(&h, user).await?;
        let channel = h.channel_open_session().await?;
        channel.request_subsystem(true, "sftp").await?;

        let sftp = russh_sftp::client::SftpSession::new(channel.into_stream())
            .await
            .with_context(|_| error::SFTPSnafu {
                about: "crate sftp session",
            })?;
        if let Some(os) = self.os {
            env = env.os(os);
        }
        let command_util = (&env).into();
        let sys = SSHSession {
            session: h,
            sftp,
            command_util,
        };

        User::new(id.clone(), self.hid, self.is_system, None, env, sys).await
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

async fn detect(h: &Handle<Client>, user: String) -> crate::Result<Environment> {
    let mut channel = h.channel_open_session().await?;
    channel.exec(true, "env").await?;
    let mut envs: Vec<u8> = Vec::new();
    loop {
        // There's an event available on the session channel
        let Some(msg) = channel.wait().await else {
            break;
        };
        if let russh::ChannelMsg::Data { data } = msg {
            envs.extend(data.iter());
        }
    }
    let mut env = Environment::new(user);
    for line in envs.split(|c| *c == b'\n') {
        let mut kv = line.splitn(2, |c| *c == b'=');
        let Some(key) = kv.next() else {
            continue;
        };
        let Some(value) = kv.next() else {
            continue;
        };
        if key.starts_with(b"HOME") {
            env = env.home(std::str::from_utf8(value).unwrap());
        } else if key.starts_with(b"XDG_SESSION_TYPE") {
            env = env.os(std::str::from_utf8(value).unwrap());
        }
    }
    Ok(env)
}
