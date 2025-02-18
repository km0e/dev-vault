use dv_api::{
    fs::{CheckInfo, DirInfo, Metadata, OpenFlags},
    process::Interactor,
    User,
};
use tracing::{debug, trace};

use crate::{
    dvl::Context,
    utils::{assert_bool, assert_option, assert_result},
};

async fn check_copy_file(
    ctx: &Context<'_>,
    src: &User,
    src_uid: &str,
    src_path: &str,
    dst_uid: &str,
    dst_path: &str,
    ts: u64,
) -> Option<bool> {
    let dst = ctx.get_user(dst_uid).await?;
    let cache = assert_result!(ctx.cache.get(dst_uid, dst_path).await, ctx.interactor);
    let res = if cache.is_some_and(|dst_ts| {
        if dst_ts != ts {
            debug!("{} != {}", dst_ts, ts);
        }
        dst_ts == ts
    }) {
        false
    } else if ctx.dry_run {
        true
    } else {
        if src.hid != dst.hid {
            let mut src = assert_result!(src.open(src_path, OpenFlags::READ).await, ctx.interactor);
            let mut dst = assert_result!(
                dst.open(dst_path, OpenFlags::WRITE | OpenFlags::CREATE)
                    .await,
                ctx.interactor
            );
            assert_result!(tokio::io::copy(&mut src, &mut dst).await, ctx.interactor);
        } else {
            let main = if src.is_system { src } else { dst };
            assert_result!(main.copy(src_path, dst_path).await, ctx.interactor);
        }
        assert_result!(ctx.cache.set(dst_uid, dst_path, ts).await, ctx.interactor);
        true
    };
    ctx.interactor
        .log(&format!(
            "[{}] {} copy {}:{} -> {}:{}",
            if ctx.dry_run { "n" } else { "a" },
            if res { "exec" } else { "skip" },
            src_uid,
            src_path,
            dst_uid,
            dst_path
        ))
        .await;
    Some(res)
}

async fn check_copy_dir(
    ctx: &Context<'_>,
    src: &User,
    src_uid: &str,
    src_path: impl Into<String>,
    dst_uid: &str,
    dst_path: impl Into<String>,
    meta: Vec<Metadata>,
) -> Option<bool> {
    let mut src_path = src_path.into();
    let mut dst_path = dst_path.into();
    let src_len = src_path.len();
    let dst_len = dst_path.len();
    let mut success = false;
    for Metadata { path, ts } in meta {
        src_path.truncate(src_len);
        src_path.push_str(&path);
        dst_path.truncate(dst_len);
        dst_path.push_str(&path);
        let res = check_copy_file(ctx, src, src_uid, &src_path, dst_uid, &dst_path, ts).await?;
        success |= res;
    }
    Some(success)
}

pub async fn copy(
    ctx: &Context<'_>,
    src_uid: impl AsRef<str>,
    src_path: impl AsRef<str>,
    dst_uid: impl AsRef<str>,
    dst_path: impl Into<String>,
) -> Option<bool> {
    let src_uid = src_uid.as_ref();
    let dst_uid = dst_uid.as_ref();
    let src_path = src_path.as_ref();
    let dst_path = dst_path.into();
    assert_bool!(!src_path.is_empty(), ctx.interactor, || {
        "src_path is empty"
    });
    assert_bool!(!dst_path.is_empty(), ctx.interactor, || {
        "dst_path is empty"
    });
    let mut dst_path = dst_path.to_string();
    trace!("copy {}:{} -> {}:{}", src_uid, src_path, dst_uid, dst_path);
    let src = ctx.get_user(src_uid).await?;
    if src_path.ends_with('/') {
        let DirInfo { path, files } = assert_result!(src.check_dir(src_path).await, ctx.interactor);
        if !dst_path.ends_with('/') {
            dst_path.push('/');
        }
        check_copy_dir(ctx, src, src_uid, path, dst_uid, dst_path, files).await
    } else {
        let info = assert_result!(src.check_path(src_path).await, ctx.interactor);
        if dst_path.ends_with('/') {
            dst_path.push_str(
                src_path
                    .rsplit_once('/')
                    .map(|(_, name)| name)
                    .unwrap_or(src_path),
            );
        };
        match info {
            CheckInfo::Dir(dir) => {
                dst_path.push('/');
                check_copy_dir(ctx, src, src_uid, dir.path, dst_uid, dst_path, dir.files).await
            }
            CheckInfo::File(file) => {
                let Metadata { path, ts } = assert_option!(
                    file.into(),
                    ctx.interactor,
                    || format!("src file {} not found", src_path)
                );
                check_copy_file(ctx, src, src_uid, &path, dst_uid, &dst_path, ts).await
            }
        }
    }
}
