use dv_api::{User, fs::OpenFlags};
use tracing::warn;

use super::dev::LRes;
pub async fn try_copy(
    src: &User,
    src_uid: &str,
    src_path: &str,
    dst: &User,
    dst_uid: &str,
    dst_path: &str,
) -> LRes<()> {
    if src_uid == dst_uid {
        if src_path == dst_path {
            warn!("src and dst is same");
        } else {
            src.copy(src_path, "", dst_path).await?;
        }
    } else if src.hid == dst.hid && {
        if !src.is_system && !dst.is_system {
            //FIXME: impl more os
            src.params.os.is_linux() || dst.params.os.is_linux()
        } else {
            true
        }
    } {
        let (main, name) = if src.is_system {
            (src, dst.params.user.as_str())
        } else {
            (dst, "")
        };
        main.copy(src_path, name, dst_path).await?;
    } else {
        let mut src = src.open(src_path, OpenFlags::READ).await?;
        let mut dst = dst
            .open(dst_path, OpenFlags::WRITE | OpenFlags::CREATE)
            .await?;
        tokio::io::copy(&mut src, &mut dst).await?;
    }
    Ok(())
}
