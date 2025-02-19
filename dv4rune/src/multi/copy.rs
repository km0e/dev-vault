use dv_api::{
    fs::{CheckInfo, DirInfo, Metadata, OpenFlags},
    process::Interactor,
    User,
};
use tracing::{debug, trace};

use super::Context;

async fn check_copy_file(
    ctx: &Context<'_>,
    src: &User,
    src_uid: &str,
    src_path: &str,
    dst_uid: &str,
    dst_path: &str,
    ts: u64,
) -> Option<bool> {
    let dst = ctx.try_get_user(dst_uid).await?;
    let cache = ctx
        .async_assert_result(ctx.cache.get(dst_uid, dst_path))
        .await?;
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
            let mut src = ctx
                .async_assert_result(src.open(src_path, OpenFlags::READ))
                .await?;
            let mut dst = ctx
                .async_assert_result(dst.open(dst_path, OpenFlags::WRITE | OpenFlags::CREATE))
                .await?;
            ctx.async_assert_result(tokio::io::copy(&mut src, &mut dst))
                .await;
        } else {
            let main = if src.is_system { src } else { dst };
            ctx.async_assert_result(main.copy(src_path, dst_path)).await;
        }
        ctx.async_assert_result(ctx.cache.set(dst_uid, dst_path, ts))
            .await;
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
    ctx.assert_bool(!src_path.is_empty(), || "src_path is empty")
        .await?;
    ctx.assert_bool(!dst_uid.is_empty(), || "dst_uid is empty")
        .await?;
    let mut dst_path = dst_path.to_string();
    trace!("copy {}:{} -> {}:{}", src_uid, src_path, dst_uid, dst_path);
    let src = ctx.try_get_user(src_uid).await?;
    if src_path.ends_with('/') {
        let DirInfo { path, files } = ctx.async_assert_result(src.check_dir(src_path)).await?;
        if !dst_path.ends_with('/') {
            dst_path.push('/');
        }
        check_copy_dir(ctx, src, src_uid, path, dst_uid, dst_path, files).await
    } else {
        let info = ctx.async_assert_result(src.check_path(src_path)).await?;
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
                check_copy_file(ctx, src, src_uid, &file.path, dst_uid, &dst_path, file.ts).await
            }
        }
    }
}
