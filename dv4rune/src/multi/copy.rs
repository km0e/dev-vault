use super::dev::*;
use dv_api::{
    fs::{CheckInfo, DirInfo, FileAttributes, Metadata},
    user::{Utf8Path, Utf8PathBuf},
    whatever,
};
use tracing::{debug, info, trace};

async fn check_copy_file(
    ctx: &Context<'_>,
    src_uid: &str,
    src_path: impl AsRef<Utf8Path>,
    dst_uid: &str,
    dst_path: impl AsRef<Utf8Path>,
    src_ts: i64,
    dst_ts: Option<i64>,
) -> LRes<bool> {
    let src_path = src_path.as_ref();
    let dst_path = dst_path.as_ref();
    let src = ctx.get_user(src_uid).await?;
    let dst = ctx.get_user(dst_uid).await?;
    let cache = ctx
        .cache
        .get(dst_uid, dst_path.as_str())
        .log(ctx.interactor)
        .await?;
    let res = if cache.is_some_and(|(dst_ver, _)| {
        if dst_ver != src_ts {
            debug!("{} != {}", dst_ver, src_ts);
        }
        dst_ver == src_ts
    }) {
        false
    } else if ctx.dry_run {
        true
    } else {
        try_copy(src, src_uid, src_path, dst, dst_uid, dst_path)
            .log(ctx.interactor)
            .await?;
        let dst_ts = match dst_ts {
            Some(ts) => ts,
            _ => dst
                .check_file(dst_path.as_str())
                .await
                .1?
                .mtime
                .ok_or_else(|| dv_api::Error::Unknown(format!("{} mtime", dst_path)))?
                .into(),
        };
        ctx.cache
            .set(dst_uid, dst_path.as_str(), src_ts, dst_ts)
            .log(ctx.interactor)
            .await?;
        true
    };
    action!(
        ctx,
        res,
        "copy {}:{} -> {}:{}",
        src_uid,
        src_path,
        dst_uid,
        dst_path
    );
    Ok(res)
}

async fn check_copy_dir(
    ctx: &Context<'_>,
    src_uid: &str,
    src_path: impl Into<Utf8PathBuf>,
    dst_uid: &str,
    dst_path: impl Into<Utf8PathBuf>,
    meta: Vec<Metadata>,
) -> LRes<bool> {
    let src_path: Utf8PathBuf = src_path.into();
    let dst_path: Utf8PathBuf = dst_path.into();
    info!(
        "check_copy_dir {}:{} -> {}:{}",
        src_uid, src_path, dst_uid, dst_path
    );
    let mut success = false;
    let mut src_file = src_path.clone();
    let mut dst_file = dst_path.clone();
    for Metadata { path, ts } in meta {
        src_file.push(&path);
        dst_file.push(&path);
        let res = check_copy_file(ctx, src_uid, &src_file, dst_uid, &dst_file, ts, None).await?;
        src_file.clone_from(&src_path);
        dst_file.clone_from(&dst_path);
        success |= res;
    }
    Ok(success)
}

pub async fn copy(
    ctx: &Context<'_>,
    src_uid: impl AsRef<str>,
    src_path: impl AsRef<str>,
    dst_uid: impl AsRef<str>,
    dst_path: impl AsRef<str>,
) -> LRes<bool> {
    let src_uid = src_uid.as_ref();
    let dst_uid = dst_uid.as_ref();
    let src_path = src_path.as_ref();
    let dst_path: &str = dst_path.as_ref();
    if src_path.is_empty() {
        ctx.interactor.log("src_path is empty").await;
    }
    if dst_path.is_empty() {
        ctx.interactor.log("dst_path is empty").await;
    }
    trace!("copy {}:{} -> {}:{}", src_uid, src_path, dst_uid, dst_path);
    let src = ctx.get_user(src_uid).await?;
    let dst = ctx.get_user(dst_uid).await?;
    let confirm = |fa: dv_api::Result<FileAttributes>, is_dir: bool| -> LRes<()> {
        match fa {
            Ok(fa) if fa.is_dir() != is_dir => {
                whatever!(
                    "{} is {}a directory",
                    dst_path,
                    is_dir.then_some("not ").unwrap_or_default()
                )
            }
            Err(e) if !e.is_not_found() => Err(e)?,
            _ => Ok(()),
        }
    };
    if src_path.ends_with('/') {
        let DirInfo { path, files } = src.check_dir(src_path).log(ctx.interactor).await?;
        let (dst_path, fa) = dst.check_file(dst_path).await;
        confirm(fa, true)?;
        check_copy_dir(ctx, src_uid, path, dst_uid, dst_path, files).await
    } else {
        let info = src.check_path(src_path).log(ctx.interactor).await?;
        let (mut dst_path2, fa) = dst.check_file(dst_path).await;

        if dst_path.ends_with('/') {
            dst_path2.push(
                src_path
                    .rsplit_once('/')
                    .map(|(_, name)| name)
                    .unwrap_or(src_path),
            );
        };
        match info {
            CheckInfo::Dir(dir) => {
                confirm(fa, true)?;
                check_copy_dir(ctx, src_uid, dir.path, dst_uid, dst_path2, dir.files).await
            }
            CheckInfo::File(file) => {
                confirm(fa, dst_path.ends_with('/'))?;
                check_copy_file(
                    ctx,
                    src_uid,
                    file.path.as_path(),
                    dst_uid,
                    dst_path2,
                    file.ts,
                    (!dst_path.ends_with('/')).then_some(file.ts),
                )
                .await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::Path, time::Duration};

    use dv_api::{LocalConfig, UserCast};
    use tokio::time::sleep;

    use crate::{cache::SqliteCache, dv::tests::TestDv, interactor::TermInteractor};

    use assert_fs::{TempDir, fixture::ChildPath, prelude::*};

    use super::copy;

    async fn tenv(src: &[(&str, &str)], dst: &[(&str, &str)]) -> (TestDv, TempDir) {
        let int = TermInteractor::new().unwrap();
        let cache = SqliteCache::memory();
        let dir = TempDir::new().unwrap();
        let user = LocalConfig {
            hid: "local".into(),
            mount: dir.to_string_lossy().to_string(),
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
        let ctx = dv.context();
        assert!(
            copy(&ctx, "this", src, "this", dst).await.unwrap(),
            "copy should success"
        );
        content_assert(&dir.child("dst"), &[("f0", "f0"), ("f1", "f1")]);
        cache_assert2(ctx.cache, dir.child("src"), dir.child("dst"), &["f0", "f1"]).await;
    }
    async fn copy_file_fixture(dst: &str, expct: &str) {
        let (dv, dir) = tenv(&[("f0", "f0")], &[]).await;
        let ctx = dv.context();
        assert!(
            copy(&ctx, "this", "src/f0", "this", dst).await.unwrap(),
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
        let ctx = dv.context();
        assert!(
            copy(&ctx, "this", "src", "this", "dst").await.unwrap(),
            "sync should success"
        );
        sleep(Duration::from_secs(2)).await;
        let src = dir.child("src");
        src.child("f0").write_str("f0").unwrap();
        src.child("f1").write_str("f1").unwrap();
        assert!(
            copy(&ctx, "this", "src/", "this", "dst").await.unwrap(),
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
        let ctx = dv.context();
        let src = dir.child("src");
        assert!(
            copy(&ctx, "this", "src/", "this", "dst").await.unwrap(),
            "sync should success"
        );
        assert!(
            !copy(&ctx, "this", "src/", "this", "dst").await.unwrap(),
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
