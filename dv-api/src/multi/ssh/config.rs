use std::sync::Arc;

use resplus::flog;
use russh::client::{self, AuthResult, Handle};
use tokio::io::AsyncReadExt;
use tracing::warn;

use crate::{Result, util::Os, whatever};

use super::{Client, SSHSession, dev::*};

#[cfg_attr(feature = "rune", derive(rune::Any))]
#[derive(Debug, Default)]
pub struct SSHConfig {
    #[cfg_attr(feature = "rune", rune(get, set))]
    pub hid: String,
    #[cfg_attr(feature = "rune", rune(get, set))]
    pub mount: String,
    #[cfg_attr(feature = "rune", rune(get, set))]
    pub host: String,
    #[cfg_attr(feature = "rune", rune(get, set))]
    pub is_system: bool,
    #[cfg_attr(feature = "rune", rune(get, set))]
    pub os: Os,
    #[cfg_attr(feature = "rune", rune(get, set))]
    pub passwd: Option<String>,
}

impl SSHConfig {
    pub fn new(hid: impl Into<String>, host: impl Into<String>) -> Self {
        Self {
            hid: hid.into(),
            host: host.into(),
            ..Default::default()
        }
    }
}
#[async_trait::async_trait]
impl UserCast for SSHConfig {
    async fn cast(self) -> Result<User> {
        let (h, user) = connect(self.host, self.passwd).await?;
        let mut p = Params::new(user);
        p.mount(self.mount);
        if !self.os.is_unknown() {
            p.os(self.os);
        }
        #[cfg(feature = "path-home")]
        let home = detect2(&h, &mut p).await?;
        #[cfg(not(feature = "path-home"))]
        detect2(&h, &mut p).await?;
        let channel = flog!(h.channel_open_session()).await?;
        flog!(channel.request_subsystem(true, "sftp")).await?;
        let sftp = russh_sftp::client::SftpSession::new(channel.into_stream()).await?;

        let command_util = (&p).into();
        let sys = SSHSession {
            session: h,
            sftp,
            #[cfg(feature = "path-home")]
            home,
            command_util,
        };

        User::new(self.hid, p, self.is_system, sys).await
    }
}

async fn connect(host: String, passwd: Option<String>) -> Result<(Handle<Client>, String)> {
    let host_cfg = flog!(russh_config::parse_home(&host), ..)?; //with host
    let config = client::Config::default();
    let config = Arc::new(config);
    let sh = Client {};

    let mut session = flog!(client::connect(
        config,
        (host_cfg.host_name.clone(), host_cfg.port),
        sh
    ))
    .await?;

    let mut res = flog!(session.authenticate_none(&host_cfg.user)).await?;
    let AuthResult::Failure {
        mut remaining_methods,
        ..
    } = res
    else {
        return Ok((session, host_cfg.user));
    };
    warn!("authenticate_none failed");
    use russh::{MethodKind, keys};
    if let (Some(path), true) = (
        host_cfg.identity_file,
        remaining_methods.contains(&MethodKind::PublicKey),
    ) {
        let kp = keys::load_secret_key(&path, None)?;
        let private_key = keys::PrivateKeyWithHashAlg::new(Arc::new(kp), None);
        res = flog!(
            session.authenticate_publickey(&host_cfg.user, private_key,),
            0
        )
        .await?;
        let AuthResult::Failure {
            remaining_methods: s,
            ..
        } = res
        else {
            return Ok((session, host_cfg.user));
        };
        warn!("authenticate_publickey with {} failed", path);
        remaining_methods = s;
    }
    if let (Some(passwd), true) = (passwd, remaining_methods.contains(&MethodKind::Password)) {
        res = flog!(session.authenticate_password(&host_cfg.user, passwd), 0).await?;
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
    )
}

#[cfg(feature = "path-home")]
type DetectResult = Result<Option<camino::Utf8PathBuf>>;

#[cfg(not(feature = "path-home"))]
type DetectResult = Result<()>;

async fn detect2(h: &Handle<Client>, p: &mut Params) -> DetectResult {
    if p.os.is_linux() {
        detect(h, p).await
    } else {
        whatever!("{} not supported", p.os)
    }
}
async fn detect(h: &Handle<Client>, p: &mut Params) -> DetectResult {
    async fn extract<const S: usize>(
        h: &Handle<Client>,
        cmd: &str,
        keys: &[&str; S],
    ) -> std::result::Result<[Option<String>; S], russh::Error> {
        let mut channel = h.channel_open_session().await?;
        channel.exec(true, cmd).await?;
        let mut output = String::with_capacity(1024);
        channel.make_reader().read_to_string(&mut output).await?;

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
        Ok(values)
    }

    #[cfg(feature = "path-home")]
    let [home, session] = flog!(extract(h, "env", &["HOME", "XDG_SESSION_TYPE"])).await?;

    #[cfg(not(feature = "path-home"))]
    let [session] = extract(h, "env", &["XDG_SESSION_TYPE"]).await?;

    if let Some(session) = session {
        p.session(session);
    }
    let [os] = extract(
        h,
        "sh -c 'cat /etc/os-release 2>/dev/null || cat /usr/lib/os-release 2>/dev/null'",
        &["ID"],
    )
    .await?;
    if let Some(os) = os {
        p.os(os);
    }
    #[cfg(feature = "path-home")]
    let res = Ok(home.map(|h| h.into()));
    #[cfg(not(feature = "path-home"))]
    let res = Ok(());
    res
}
