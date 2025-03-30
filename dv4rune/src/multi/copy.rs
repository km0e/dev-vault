use std::{borrow::Cow, ops::Deref};

use super::dev::*;
use dv_api::{
    User,
    fs::{CheckInfo, DirInfo, FileAttributes, Metadata},
    user::{Utf8Path, Utf8PathBuf},
    whatever,
};
use tracing::{debug, trace};

pub struct CopyContext<'a> {
    ctx: Context<'a>,
    src: &'a User,
    src_uid: &'a str,
    dst: &'a User,
    dst_uid: &'a str,
    opt: Option<&'a str>,
}

impl<'a> Deref for CopyContext<'a> {
    type Target = Context<'a>;
    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}
impl<'a> CopyContext<'a> {
    pub async fn new(
        ctx: Context<'a>,
        src_uid: &'a str,
        dst_uid: &'a str,
        mut opt: Option<&'a str>,
    ) -> LRes<Self> {
        let src = ctx.get_user(src_uid).await?;
        let dst = ctx.get_user(dst_uid).await?;
        if let Some(_opt) = opt {
            if !matches!(_opt, "o" | "n" | "u") {
                ctx.interactor
                    .log(format!("invalid option: {}", _opt))
                    .await;
                opt = None;
            }
        }
        Ok(Self {
            ctx,
            src,
            src_uid,
            dst,
            dst_uid,
            opt,
        })
    }

    async fn check_copy_file(
        &self,
        src_path: &Utf8Path,
        dst_path: &Utf8Path,
        src_ts: i64,
        dst_ts: Option<i64>,
    ) -> LRes<bool> {
        trace!(
            "check_copy_file {}:{} -> {}:{}",
            self.src_uid, src_path, self.dst_uid, dst_path
        );
        let cache = self
            .ctx
            .cache
            .get(self.dst_uid, dst_path.as_str())
            .log(self.interactor)
            .await?;
        let (do_, rev) = match (cache, dst_ts) {
            (Some((dst_ver, old_dst_ts)), Some(dst_ts))
                if dst_ver == src_ts || old_dst_ts == dst_ts =>
            {
                debug!(
                    "src {{path:{} old_ts:{} ts:{}}} dst {{path:{} old_ts:{} ts:{}}}",
                    src_path, dst_ver, src_ts, dst_path, old_dst_ts, dst_ts
                );
                if old_dst_ts != dst_ts {
                    match self.opt {
                        Some("n") => (false, false),
                        Some("u") => (true, true),
                        _ => {
                            let sel = self
                                .interactor
                                .confirm(
                                    format!("{} is newer, do what?", dst_path),
                                    &["n/skip", "u/update"],
                                )
                                .log(self.interactor)
                                .await?;
                            (sel == 1, true)
                        }
                    }
                } else {
                    (dst_ver != src_ts, false)
                }
            }
            (_, None) => (true, false),
            _ => match self.opt {
                Some("o") => (true, false),
                Some("n") => (false, false),
                Some("u") => (true, true),
                _ => {
                    let sel = self
                        .interactor
                        .confirm(
                            format!("{} is newer, do what?", dst_path),
                            &["y/overwrite", "n/skip", "u/update"],
                        )
                        .log(self.interactor)
                        .await?;
                    (sel != 1, sel == 2)
                }
            },
        };
        if do_ && !self.dry_run {
            let (src_ts, dst_ts) = if !rev {
                try_copy(
                    self.src,
                    self.src_uid,
                    src_path,
                    self.dst,
                    self.dst_uid,
                    dst_path,
                )
                .await?;
                (Some(src_ts), self.dst.get_mtime(dst_path).await?)
            } else {
                try_copy(
                    self.dst,
                    self.dst_uid,
                    dst_path,
                    self.src,
                    self.src_uid,
                    src_path,
                )
                .await?;
                (self.src.get_mtime(src_path).await?, dst_ts)
            };
            let Some(src_ts) = src_ts else {
                whatever!("get {} mtime failed", src_path)
            };
            let Some(dst_ts) = dst_ts else {
                whatever!("get {} mtime failed", dst_path)
            };
            self.cache
                .set(self.dst_uid, dst_path.as_str(), src_ts, dst_ts)
                .log(self.interactor)
                .await?;
        }
        action!(
            self,
            do_,
            "{} {}:{} {} {}:{}",
            if do_ && rev { "update" } else { "copy" },
            self.src_uid,
            src_path,
            if do_ && rev { "<-" } else { "->" },
            self.dst_uid,
            dst_path
        );
        Ok(do_)
    }

    async fn check_copy_dir(
        &self,
        src_path: Utf8PathBuf,
        dst_path: Utf8PathBuf,
        meta: Vec<Metadata>,
    ) -> LRes<bool> {
        let mut success = false;
        let mut src_file = src_path.to_string();
        let mut dst_file = dst_path.to_string();
        let src_len = src_file.len();
        let dst_len = dst_file.len();
        for Metadata { path, ts } in meta {
            src_file.push('/'); //NOTE: avoid using '\' in path
            src_file.push_str(path.as_str());
            dst_file.push('/');
            dst_file.push_str(path.as_str());
            let dst_ts = self.dst.get_mtime(dst_file.as_str().into()).await?;
            let res = self
                .check_copy_file(
                    src_file.as_str().into(),
                    dst_file.as_str().into(),
                    ts,
                    dst_ts,
                )
                .await?;
            src_file.truncate(src_len);
            dst_file.truncate(dst_len);
            success |= res;
        }
        Ok(success)
    }

    pub async fn copy(&self, src_path: impl AsRef<str>, dst_path: impl AsRef<str>) -> LRes<bool> {
        let src_path = src_path.as_ref();
        let dst_path: &str = dst_path.as_ref();
        trace!(
            "copy {}:{} -> {}:{}",
            self.src_uid, src_path, self.dst_uid, dst_path
        );
        let confirm = |fa: dv_api::Result<FileAttributes>, is_dir: bool| -> LRes<Option<i64>> {
            match fa {
                Ok(fa) if fa.is_dir() != is_dir => {
                    whatever!(
                        "{} is {}a directory",
                        dst_path,
                        is_dir.then_some("not ").unwrap_or_default()
                    )
                }
                Err(e) if !e.is_not_found() => Err(e)?,
                Ok(fa) => Ok(Some(fa.mtime.unwrap_or_default().into())),
                _ => Ok(None),
            }
        };
        if src_path.ends_with('/') {
            let DirInfo { path, files } = self.src.check_dir(src_path).log(self.interactor).await?;
            let (dst_path, fa) = self.dst.check_file(dst_path.into()).await;
            confirm(fa, true)?;
            self.check_copy_dir(path, dst_path, files).await
        } else {
            let info = self.src.check_path(src_path).log(self.interactor).await?;
            let dst_path2 = if dst_path.ends_with('/') {
                format!(
                    "{}{}",
                    dst_path,
                    src_path
                        .rsplit_once('/')
                        .map(|(_, name)| name)
                        .unwrap_or(src_path)
                )
                .into()
            } else {
                Cow::Borrowed(dst_path)
            };
            let (dst_path2, fa) = self.dst.check_file(dst_path2.as_ref().into()).await;
            match info {
                CheckInfo::Dir(dir) => {
                    confirm(fa, true)?;
                    self.check_copy_dir(dir.path, dst_path2, dir.files).await
                }
                CheckInfo::File(file) => {
                    let dst_ts = confirm(fa, false)?;
                    self.check_copy_file(&file.path, &dst_path2, file.ts, dst_ts)
                        .await
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::Path, time::Duration};

    use dv_api::Config;
    use tokio::time::sleep;

    use crate::{cache::SqliteCache, dv::tests::TestDv, interactor::TermInteractor};

    use assert_fs::{TempDir, fixture::ChildPath, prelude::*};

    use super::CopyContext;

    async fn tenv(src: &[(&str, &str)], dst: &[(&str, &str)]) -> (TestDv, TempDir) {
        let int = TermInteractor::new().unwrap();
        let cache = SqliteCache::memory();
        let dir = TempDir::new().unwrap();
        let mut cfg = Config::default();
        cfg.insert("MOUNT", dir.to_string_lossy());
        let mut users = HashMap::new();
        users.insert("this".to_string(), cfg.connect(None).await.unwrap());
        let src_dir = dir.child("src");
        for (name, content) in src {
            let f = src_dir.child(name);
            f.write_str(content).unwrap();
        }
        let dst_dir = dir.child("dst");
        for (name, content) in dst {
            let f = dst_dir.child(name);
            f.write_str(content).unwrap();
        }
        (
            TestDv {
                dry_run: false,
                users,
                cache,
                interactor: int,
            },
            dir,
        )
    }
    fn content_assert(dir: &ChildPath, pairs: &[(&str, &str)]) {
        for (name, content) in pairs {
            dir.child(name).assert(*content);
        }
    }
    async fn cache_assert(cache: &SqliteCache, src: &Path, dst: &Path) {
        let src_meta = src.metadata().unwrap();
        let dst_meta = dst.metadata().unwrap();
        let mtime = {
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                (
                    src_meta.last_write_time() as i64,
                    dst_meta.last_write_time() as i64,
                )
            }
            #[cfg(not(windows))]
            {
                use std::os::unix::fs::MetadataExt;
                (src_meta.mtime(), dst_meta.mtime())
            }
        };
        assert_eq!(
            mtime,
            cache
                .get("this", dst.to_str().unwrap())
                .await
                .unwrap()
                .unwrap(),
            "about path: {}",
            dst.display()
        );
    }
    async fn cache_assert2(cache: &SqliteCache, src: ChildPath, dst: ChildPath, subpaths: &[&str]) {
        for subpath in subpaths {
            cache_assert(cache, src.child(subpath).path(), dst.child(subpath).path()).await;
        }
    }
    async fn copy_dir_fixture(src: &str, dst: &str) {
        let (dv, dir) = tenv(&[("f0", "f0"), ("f1", "f1")], &[]).await;
        let ctx = CopyContext::new(dv.context(), "this", "this", Some("o"))
            .await
            .unwrap();
        assert!(ctx.copy(src, dst).await.unwrap(), "copy should success");
        content_assert(&dir.child("dst"), &[("f0", "f0"), ("f1", "f1")]);
        cache_assert2(ctx.cache, dir.child("src"), dir.child("dst"), &["f0", "f1"]).await;
    }
    async fn copy_file_fixture(dst: &str, expct: &str) {
        let (dv, dir) = tenv(&[("f0", "f0")], &[]).await;
        let ctx = CopyContext::new(dv.context(), "this", "this", Some("o"))
            .await
            .unwrap();
        assert!(
            ctx.copy("src/f0", dst).await.unwrap(),
            "copy should success"
        );
        dir.child(expct).assert("f0");
        cache_assert(
            ctx.cache,
            dir.child("src/f0").path(),
            dir.child(expct).path(),
        )
        .await;
    }
    #[tokio::test]
    async fn copy_dir() {
        copy_dir_fixture("src/", "dst").await;
        copy_dir_fixture("src/", "dst/").await;
        copy_dir_fixture("src", "dst").await;
        copy_dir_fixture("src", "dst").await;
    }
    #[tokio::test]
    async fn copy_file() {
        copy_file_fixture("dst", "dst").await;
        copy_file_fixture("dst/", "dst/f0").await;
    }
    #[tokio::test]
    async fn test_update() {
        let (dv, dir) = tenv(&[("f0", "f00"), ("f1", "f11")], &[]).await;
        let ctx = CopyContext::new(dv.context(), "this", "this", Some("o"))
            .await
            .unwrap();
        assert!(ctx.copy("src", "dst").await.unwrap(), "sync should success");
        sleep(Duration::from_secs(2)).await;
        let src = dir.child("src");
        src.child("f0").write_str("f0").unwrap();
        src.child("f1").write_str("f1").unwrap();
        assert!(
            ctx.copy("src/", "dst").await.unwrap(),
            "sync should success"
        );
        let dst = dir.child("dst");
        dst.child("f0").assert("f0");
        dst.child("f1").assert("f1");
        cache_assert(ctx.cache, src.child("f0").path(), dst.child("f0").path()).await;
        cache_assert(ctx.cache, src.child("f1").path(), dst.child("f1").path()).await;
    }
    #[tokio::test]
    async fn test_donothing() {
        let (dv, dir) = tenv(&[("f0", "f0"), ("f1", "f1")], &[]).await;
        let ctx = CopyContext::new(dv.context(), "this", "this", Some("o"))
            .await
            .unwrap();
        let src = dir.child("src");
        assert!(
            ctx.copy("src/", "dst").await.unwrap(),
            "sync should success"
        );
        assert!(
            !ctx.copy("src/", "dst").await.unwrap(),
            "sync should do nothing"
        );
        src.child("f0").assert("f0");
        src.child("f1").assert("f1");
        cache_assert(
            ctx.cache,
            dir.child("src/f0").path(),
            dir.child("dst/f0").path(),
        )
        .await;
        cache_assert(
            ctx.cache,
            dir.child("src/f1").path(),
            dir.child("dst/f1").path(),
        )
        .await;
    }
}
