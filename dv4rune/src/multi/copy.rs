use dv_api::{
    fs::{CheckInfo, DirInfo, Metadata, OpenFlags},
    process::Interactor,
    User,
};
use rune::support::Result as LRes;
use tracing::{debug, info, trace};

use crate::utils::LogFutResult;

use super::Context;

async fn check_copy_file(
    ctx: &Context<'_>,
    src: &User,
    src_uid: &str,
    src_path: &str,
    dst_uid: &str,
    dst_path: &str,
    ts: u64,
) -> LRes<bool> {
    let dst = ctx.get_user(dst_uid).await?;
    let cache = ctx.cache.get(dst_uid, dst_path).log(ctx.interactor).await?;
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
        ctx.cache
            .set(dst_uid, dst_path, ts)
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
    src: &User,
    src_uid: &str,
    src_path: impl Into<String>,
    dst_uid: &str,
    dst_path: impl Into<String>,
    meta: Vec<Metadata>,
) -> LRes<bool> {
    let mut src_path = src_path.into();
    let mut dst_path = dst_path.into();
    info!(
        "check_copy_dir {}:{} -> {}:{}",
        src_uid, src_path, dst_uid, dst_path
    );
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
    Ok(success)
}

pub async fn copy(
    ctx: &Context<'_>,
    src_uid: impl AsRef<str>,
    src_path: impl AsRef<str>,
    dst_uid: impl AsRef<str>,
    dst_path: impl Into<String>,
) -> LRes<bool> {
    let src_uid = src_uid.as_ref();
    let dst_uid = dst_uid.as_ref();
    let src_path = src_path.as_ref();
    let dst_path = dst_path.into();
    if src_path.is_empty() {
        ctx.interactor.log("src_path is empty").await;
    }
    if dst_path.is_empty() {
        ctx.interactor.log("dst_path is empty").await;
    }
    let mut dst_path = dst_path.to_string();
    trace!("copy {}:{} -> {}:{}", src_uid, src_path, dst_uid, dst_path);
    let src = ctx.get_user(src_uid).await?;
    if src_path.ends_with('/') {
        let DirInfo { path, files } = src.check_dir(src_path).log(ctx.interactor).await?;
        if !dst_path.ends_with('/') {
            dst_path.push('/');
        }
        check_copy_dir(ctx, src, src_uid, path, dst_uid, dst_path, files).await
    } else {
        let info = src.check_path(src_path).log(ctx.interactor).await?;
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
