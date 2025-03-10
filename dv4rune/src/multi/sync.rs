use super::dev::*;
use dv_api::fs::{CheckInfo, Metadata};
use tracing::trace;

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

    #[derive(Default)]
    struct CopyItem {
        src_path: String,
        dst_path: String,
        skip: bool,
        reverse: bool,
        ts: Option<i64>,
    }

    impl CopyItem {
        fn new(src_path: String, dst_path: String) -> Self {
            Self {
                src_path,
                dst_path,
                ..Default::default()
            }
        }
        fn ts(mut self, ts: i64) -> Self {
            self.ts = Some(ts);
            self
        }
        fn skip(mut self) -> Self {
            self.skip = true;
            self
        }
        fn reverse(mut self) -> Self {
            self.reverse = true;
            self
        }
    }

    let copy_file = async |src_path: String, dst_path: String| -> LRes<CopyItem> {
        let res = dst.check_file(&dst_path).await;
        let res = match res {
            Err(e) if e.is_not_found() => CopyItem::new(src_path, dst_path),
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
                let mut item = CopyItem::new(src_path, dst_path).reverse();
                if cache_ts.is_some_and(|v| v == ts) {
                    item = item.skip();
                } else if !ctx.dry_run {
                    item = item.ts(ts);
                }
                item
            }
            Err(e) => Err(e)?,
        };
        Ok(res)
    };
    let res = src.check_path(src_path).log(ctx.interactor).await?;
    let copy_items = match res {
        CheckInfo::Dir(mut di) => {
            if !dst_path.ends_with('/') {
                dst_path.push('/');
            }
            let mut copy_items = Vec::new();
            if !di.path.ends_with('/') {
                di.path.push('/');
            }
            let src_path = di.path;
            if di.files.is_empty() {
                let di = dst.check_dir(&dst_path).await?;
                let dst_path = di.path;
                for Metadata { path, ts } in di.files {
                    let dst_path = format!("{}{}", dst_path, path);
                    let src_path = format!("{}{}", src_path, path);
                    copy_items.push(CopyItem::new(src_path, dst_path).ts(ts).reverse());
                }
            } else {
                for Metadata { path, .. } in di.files {
                    let dst_path = format!("{}{}", dst_path, path);
                    let src_path = format!("{}{}", src_path, path);
                    let res = copy_file(src_path, dst_path).await?;
                    copy_items.push(res);
                }
            }
            copy_items
        }
        CheckInfo::File(m) => {
            vec![copy_file(m.path, dst_path).await?]
        }
    };
    let mut res = false;
    for CopyItem {
        src_path,
        mut dst_path,
        skip,
        reverse,
        ts,
    } in copy_items
    {
        if !ctx.dry_run {
            if reverse {
                try_copy(dst, dst_uid, &dst_path, src, src_uid, &src_path).await?;
            } else {
                try_copy(src, src_uid, &src_path, dst, dst_uid, &dst_path).await?;
            }
            let ts = match ts {
                Some(ts) => ts,
                None => {
                    let (p, fa) = dst.check_file(&dst_path).await?;
                    dst_path = p;
                    fa.mtime
                        .ok_or_else(|| rune::support::Error::msg("get version fail"))?
                        as i64
                }
            };
            ctx.cache
                .set(dst_uid, &dst_path, ts)
                .log(ctx.interactor)
                .await?;
        }
        let (src_uid, src_path, dst_uid, dst_path) = if reverse {
            (dst_uid, &dst_path, src_uid, &src_path)
        } else {
            (src_uid, &src_path, dst_uid, &dst_path)
        };
        action!(
            ctx,
            !skip,
            "{}",
            format!("copy {}:{} -> {}:{}", src_uid, src_path, dst_uid, dst_path)
        );
        res |= !skip;
    }
    Ok(res)
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, fs::create_dir, path::Path, time::Duration};

    use dv_api::{LocalConfig, User, UserCast};
    use tokio::time::sleep;

    use crate::{cache::SqliteCache, interactor::TermInteractor, multi::Context};

    use assert_fs::{TempDir, prelude::*};

    use super::sync;

    async fn tenv(
        src: &[(&str, &str)],
        dst: &[(&str, &str)],
    ) -> (TermInteractor, SqliteCache, HashMap<String, User>, TempDir) {
        let int = TermInteractor::new().unwrap();
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
        let metadata = path.metadata().unwrap();
        let mtime = {
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                metadata.last_write_time() as i64
            }
            #[cfg(not(windows))]
            {
                use std::os::unix::fs::MetadataExt;
                metadata.mtime()
            }
        };
        assert_eq!(
            mtime,
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
    async fn test_src_to_dst2() {
        let (int, cache, users, dir) = tenv(&[("f0", "f0"), ("f1", "f1")], &[]).await;
        let ctx = Context::new(false, &cache, &int, &users);
        let dst = dir.child("dst");
        assert!(
            sync(&ctx, "this", "src", "this", "dst").await.unwrap(),
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
