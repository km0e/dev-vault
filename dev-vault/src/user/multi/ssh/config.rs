use std::sync::Arc;

use russh::client::{self, Handle};
use snafu::{whatever, ResultExt};
use tracing::warn;

use crate::error;

use super::{dev::*, Client, SSHSession};

#[derive(Debug)]
pub struct SSHUserConfig {
    pub uid: String,
    pub hid: String,
    pub is_system: bool,
    pub os: Option<String>,
    pub host: String,
    pub passwd: Option<String>,
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

        let dev = User::new(id.clone(), self.hid, self.is_system, None, env, sys);
        Ok(dev)
    }
}

async fn by_publickey(
    session: &mut Handle<Client>,
    identity_file: Option<String>,
    user: impl Into<String>,
) -> crate::Result<bool> {
    if let Some(path) = identity_file {
        let kp = russh_keys::load_secret_key(&path, None)?;
        let auth_res = session
            .authenticate_publickey(
                user,
                russh_keys::key::PrivateKeyWithHashAlg::new(Arc::new(kp), None)?,
            )
            .await?;
        if !auth_res {
            warn!("authenticate_publickey with {} failed", path);
        }
        Ok(auth_res)
    } else {
        Ok(false)
    }
}
async fn by_password(
    session: &mut Handle<Client>,
    passwd: Option<String>,
    user: impl Into<String>,
) -> crate::Result<bool> {
    if let Some(passwd) = passwd {
        let auth_res = session.authenticate_password(user, passwd).await?;
        if !auth_res {
            warn!("authenticate_password failed");
        }
        Ok(auth_res)
    } else {
        Ok(false)
    }
}

async fn connect(host: String, passwd: Option<String>) -> crate::Result<(Handle<Client>, String)> {
    let host_cfg = russh_config::parse_home(&host)?;
    let config = client::Config::default();
    let config = Arc::new(config);
    let sh = Client {};

    let mut session =
        client::connect(config, (host_cfg.host_name.clone(), host_cfg.port), sh).await?;
    if !by_publickey(&mut session, host_cfg.identity_file, &host_cfg.user).await?
        && !by_password(&mut session, passwd, &host_cfg.user).await?
    {
        whatever!(
            "ssh connect {} {} {} failed",
            host,
            host_cfg.host_name,
            host_cfg.user
        );
    }
    Ok((session, host_cfg.user))
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
