use super::dev::*;
use dv_api::{
    fs::{CheckInfo, DirInfo, Metadata},
    User,
};
use std::borrow::Cow;
use tracing::{debug, info, trace};

pub struct CopyItem<'a, 'b> {
    pub src: PathDetail<'a, 'b>,
    pub dst: PathDetail<'a, 'b>,
    pub version: u64,
}

pub struct PathDetail<'a, 'b> {
    pub user: &'a User,
    pub path: Cow<'b, str>,
}

impl<'a, 'b> PathDetail<'a, 'b> {
    pub fn new(user: &'a User, path: impl Into<Cow<'b, str>>) -> Self {
        Self {
            user,
            path: path.into(),
        }
    }
}

async fn check_file<'a, I: ContextImpl>(
    dst_uid: &str,
    dst: &PathDetail<'a, '_>,
    src_ts: u64,
    context: &'a Context<I>,
) -> Result<bool> {
    let cache = context.get_cache();
    let interactor = context.get_interactor();
    if cache
        .get(dst_uid, &dst.path)
        .await?
        .is_some_and(|v| v == src_ts)
    {
        interactor
            .log(&format!("[Skip] copy to {}:{}", &dst_uid, &dst.path))
            .await;
        return Ok(false);
    }

    info!("[Task] {} needs to be updated to {}", dst.path, src_ts);
    trace!("check over");
    Ok(true)
}
async fn check_copy_file<'a, 'b, I: ContextImpl>(
    src_user: &'a User,
    dst_uid: &str,
    dst_user: &'a User,
    metadata: Metadata,
    dst_path: &'b String,
    context: &'a Context<I>,
) -> Result<Option<CopyItem<'a, 'b>>> {
    let dst = PathDetail::new(dst_user, dst_path);
    check_file(dst_uid, &dst, metadata.ts, context)
        .await
        .map(|update| {
            update.then(|| CopyItem {
                src: PathDetail::new(src_user, metadata.path),
                dst,
                version: metadata.ts,
            })
        })
}
async fn check_copy_dir<'a, 'b, I: ContextImpl>(
    src_user: &'a User,
    dst_uid: &str,
    dst_user: &'a User,
    dir: DirInfo,
    dst_path: &'b String,
    context: &'a Context<I>,
) -> Result<Vec<CopyItem<'a, 'b>>> {
    let mut copy_info = Vec::new();
    for Metadata { path, ts } in dir.files {
        let dst = PathDetail::new(dst_user, format!("{}/{}", dst_path, path));
        if check_file(dst_uid, &dst, ts, context).await? {
            copy_info.push(CopyItem {
                src: PathDetail::new(src_user, format!("{}/{}", dir.path, path)),
                dst,
                version: ts,
            });
        }
    }
    Ok(copy_info)
}
pub async fn check<'a, 'b, 'c, I: ContextImpl>(
    target: &'c Target,
    inner: &'b super::CopyInner,
    context: &'a Context<I>,
) -> Result<(&'c str, &'c str, Vec<CopyItem<'a, 'b>>)> {
    let mut copy_info = Vec::new();
    let (src_uid, dst_uid) = target.get_uid()?;
    let src_user = context.get_user(src_uid, false)?;
    let dst_user = context.get_user(dst_uid, false)?;

    for (src, dst) in inner.pair.iter() {
        let ck_res = src_user.check_path(src).await?;
        debug!("check {src} -> {dst}");
        match ck_res {
            CheckInfo::File(file) => {
                let copy = check_copy_file(src_user, dst_uid, dst_user, file, dst, context).await?;
                copy_info.extend(copy);
            }
            CheckInfo::Dir(info) => {
                let copies =
                    check_copy_dir(src_user, dst_uid, dst_user, info, dst, context).await?;
                copy_info.extend(copies);
            }
        }
    }
    trace!("check over");
    Ok((src_uid, dst_uid, copy_info))
}
