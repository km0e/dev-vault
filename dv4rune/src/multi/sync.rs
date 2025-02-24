use dv_api::{
    fs::{CheckInfo, Metadata, OpenFlags},
    process::Interactor,
};
use rune::support::Result as LRes;
use tracing::trace;

use crate::utils::LogFutResult;

use super::Context;

pub async fn sync(
    ctx: &Context<'_>,
    src_uid: impl AsRef<str>,
    src_path: impl AsRef<str>,
    dst_uid: impl AsRef<str>,
    dst_path: impl Into<String>,
) -> LRes<bool> {
    let src_uid = src_uid.as_ref();
    let dst_uid = dst_uid.as_ref();
    let src_path = src_path.as_ref();
    let mut dst_path: String = dst_path.into();
    if src_path.is_empty() {
        ctx.interactor.log("src_path is empty").await;
    }
    if dst_path.is_empty() {
        ctx.interactor.log("dst_path is empty").await;
    }
    trace!("copy {}:{} -> {}:{}", src_uid, src_path, dst_uid, dst_path);
    let src = ctx.get_user(src_uid).await?;
    let dst = ctx.get_user(dst_uid).await?;

    let copy_file = async |src_path: &str, dst_path: &str| -> LRes<bool> {
        let res = dst.check_file(dst_path).await;
        let res = match res {
            Err(e) if e.is_not_found() => {
                if ctx.dry_run {
                    true
                } else {
                    if src.hid != dst.hid {
                        let mut src = src
                            .open(src_path, OpenFlags::READ)
                            .log(ctx.interactor)
                            .await?;
                        let mut dst = dst
                            .open(dst_path, OpenFlags::WRITE | OpenFlags::CREATE)
                            .log(ctx.interactor)
                            .await?;
                        tokio::io::copy(&mut src, &mut dst)
                            .log(ctx.interactor)
                            .await?;
                    } else {
                        let main = if src.is_system { src } else { dst };
                        main.copy(src_path, dst_path).log(ctx.interactor).await?;
                    }
                    let (dst_path, fa) = dst.check_file(dst_path).await?;

                    let Some(ts) = fa.mtime else {
                        Err(rune::support::Error::msg("get version fail"))?
                    };

                    let ts = ts as i64;

                    ctx.cache
                        .set(dst_uid, &dst_path, ts)
                        .log(ctx.interactor)
                        .await?;

                    true
                }
            }
            Ok((dst_path, fa)) => {
                let Some(ts) = fa.mtime else {
                    Err(rune::support::Error::msg("get version fail"))?
                };

                let ts = ts as i64;

                let cache_ts = ctx
                    .cache
                    .get(dst_uid, &dst_path)
                    .log(ctx.interactor)
                    .await?;
                if cache_ts.is_some_and(|v| v == ts) {
                    false
                } else if ctx.dry_run {
                    true
                } else {
                    if src.hid != dst.hid {
                        let mut src = src
                            .open(src_path, OpenFlags::READ)
                            .log(ctx.interactor)
                            .await?;
                        let mut dst = dst
                            .open(&dst_path, OpenFlags::WRITE | OpenFlags::CREATE)
                            .log(ctx.interactor)
                            .await?;
                        tokio::io::copy(&mut dst, &mut src)
                            .log(ctx.interactor)
                            .await?;
                    } else {
                        let main = if src.is_system { src } else { dst };
                        main.copy(&dst_path, src_path).log(ctx.interactor).await?;
                    }
                    ctx.cache
                        .set(dst_uid, &dst_path, ts)
                        .log(ctx.interactor)
                        .await?;
                    true
                }
            }
            Err(e) => Err(e)?,
        };
        Ok(res)
    };
    let res = src.check_path(src_path).log(ctx.interactor).await?;
    match res {
        CheckInfo::Dir(di) => {
            if !dst_path.ends_with('/') {
                dst_path.push('/');
            }
            if di.files.is_empty() {
                let di = dst.check_dir(&dst_path).await?;
                let mut success = false;
                let mut src_path = src_path.to_string();
                let mut dst_path = di.path;
                let dst_len = dst_path.len();
                let src_len = src_path.len();
                for Metadata { path, ts } in di.files {
                    dst_path.truncate(dst_len);
                    dst_path.push_str(&path);
                    src_path.truncate(src_len);
                    src_path.push_str(&path);
                    let res = if ctx.dry_run {
                        true
                    } else {
                        if src.hid != dst.hid {
                            let mut src = src
                                .open(&src_path, OpenFlags::READ)
                                .log(ctx.interactor)
                                .await?;
                            let mut dst = dst
                                .open(&dst_path, OpenFlags::WRITE | OpenFlags::CREATE)
                                .log(ctx.interactor)
                                .await?;
                            tokio::io::copy(&mut dst, &mut src)
                                .log(ctx.interactor)
                                .await?;
                        } else {
                            let main = if src.is_system { src } else { dst };
                            main.copy(&dst_path, &src_path).log(ctx.interactor).await?;
                        }
                        ctx.cache
                            .set(dst_uid, &dst_path, ts)
                            .log(ctx.interactor)
                            .await?;

                        true
                    };
                    success |= res;
                }
                Ok(success)
            } else {
                let mut success = false;
                let mut src_path = src_path.to_string();
                let dst_len = dst_path.len();
                let src_len = src_path.len();
                for Metadata { path, .. } in di.files {
                    dst_path.truncate(dst_len);
                    dst_path.push_str(&path);
                    src_path.truncate(src_len);
                    src_path.push_str(&path);
                    let res = copy_file(&src_path, &dst_path).await?;
                    success |= res;
                }
                Ok(success)
            }
        }
        CheckInfo::File(m) => copy_file(&m.path, &dst_path).await,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap, fs::create_dir, os::unix::fs::MetadataExt, path::Path, time::Duration,
    };

    use dv_api::{LocalConfig, User, UserCast};
    use tokio::time::sleep;

    use crate::{cache::SqliteCache, interactor::TermInteractor, multi::Context};

    use assert_fs::{TempDir, prelude::*};

    use super::sync;

    async fn tenv(
        src: &[(&str, &str)],
        dst: &[(&str, &str)],
    ) -> (TermInteractor, SqliteCache, HashMap<String, User>, TempDir) {
        let int = TermInteractor::default();
        let cache = SqliteCache::memory();
        let dir = TempDir::new().unwrap();
        let user = LocalConfig {
            hid: "local".into(),
            mount: dir.to_path_buf().try_into().unwrap(),
        };
        let mut users = HashMap::new();
        users.insert("this".to_string(), user.cast().await.unwrap());
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
        (int, cache, users, dir)
    }
    async fn cache_assert(cache: &SqliteCache, path: &Path) {
        assert_eq!(
            path.metadata().unwrap().mtime(),
            cache
                .get("this", path.to_str().unwrap())
                .await
                .unwrap()
                .unwrap(),
            "about path: {}",
            path.display()
        );
    }
    #[tokio::test]
    async fn test_src_to_dst() {
        let (int, cache, users, dir) = tenv(&[("f0", "f0"), ("f1", "f1")], &[]).await;
        let ctx = Context::new(false, &cache, &int, &users);
        let dst = dir.child("dst");
        assert!(
            sync(&ctx, "this", "src/", "this", "dst").await.unwrap(),
            "sync should success"
        );
        dst.child("f0").assert("f0");
        dst.child("f1").assert("f1");
        cache_assert(&cache, dst.child("f0").path()).await;
        cache_assert(&cache, dst.child("f1").path()).await;
    }
    #[tokio::test]
    async fn test_dst_to_src() {
        let (int, cache, users, dir) = tenv(&[], &[("f0", "f0"), ("f1", "f1")]).await;
        let ctx = Context::new(false, &cache, &int, &users);
        let src = dir.child("src");
        create_dir(&src).unwrap();
        assert!(
            sync(&ctx, "this", "src/", "this", "dst").await.unwrap(),
            "sync should success"
        );
        src.child("f0").assert("f0");
        src.child("f1").assert("f1");
        let dst = dir.child("dst");
        cache_assert(&cache, dst.child("f0").path()).await;
        cache_assert(&cache, dst.child("f1").path()).await;
    }
    #[tokio::test]
    async fn test_update() {
        let (int, cache, users, dir) = tenv(&[("f0", "f"), ("f1", "f")], &[]).await;
        let ctx = Context::new(false, &cache, &int, &users);
        assert!(
            sync(&ctx, "this", "src/", "this", "dst").await.unwrap(),
            "sync should success"
        );
        sleep(Duration::from_secs(2)).await;
        let dst = dir.child("dst");
        dst.child("f0").write_str("f0").unwrap();
        dst.child("f1").write_str("f1").unwrap();
        assert!(
            sync(&ctx, "this", "src/", "this", "dst").await.unwrap(),
            "sync should success"
        );
        let src = dir.child("src");
        src.child("f0").assert("f0");
        src.child("f1").assert("f1");
        cache_assert(&cache, dst.child("f0").path()).await;
        cache_assert(&cache, dst.child("f1").path()).await;
    }
    #[tokio::test]
    async fn test_donothing() {
        let (int, cache, users, dir) = tenv(&[], &[("f0", "f0"), ("f1", "f1")]).await;
        let ctx = Context::new(false, &cache, &int, &users);
        let src = dir.child("src");
        create_dir(&src).unwrap();
        assert!(
            sync(&ctx, "this", "src/", "this", "dst").await.unwrap(),
            "sync should success"
        );
        assert!(
            !sync(&ctx, "this", "src/", "this", "dst").await.unwrap(),
            "sync should do nothing"
        );
        src.child("f0").assert("f0");
        src.child("f1").assert("f1");
        cache_assert(&cache, dir.child("dst/f0").path()).await;
        cache_assert(&cache, dir.child("dst/f1").path()).await;
    }
}
