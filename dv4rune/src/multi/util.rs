use dv_api::{User, fs::OpenFlags, user::Utf8Path, whatever};
use tracing::{info, warn};

use super::dev::LRes;
pub async fn try_copy(
    src: &User,
    src_uid: &str,
    src_path: &Utf8Path,
    dst: &User,
    dst_uid: &str,
    dst_path: &Utf8Path,
) -> LRes<()> {
    if src_uid == dst_uid {
        if src_path == dst_path {
            warn!("src and dst is same");
        } else {
            src.copy(src_path, "", dst_path).await?;
        }
    } else if matches!((src.variables.get("HID"), dst.variables.get("HID")),(Some(src_hid), Some(dst_hid)) if src_hid == dst_hid)
        && {
            if !src.is_system && !dst.is_system {
                //FIXME: impl more os
                src.dev.os.is_linux() || dst.dev.os.is_linux()
            } else {
                true
            }
        }
    {
        let (main, name) = if src.is_system {
            info!("same hid, use src to copy");

            let Some(user) = dst.variables.get("USER") else {
                whatever!("no user for dst:{}", dst_uid)
            };
            (src, user.as_str())
        } else {
            info!("same hid, use dst to copy");
            (dst, "")
        };
        main.copy(src_path, name, dst_path).await?;
    } else {
        let mut src = src.open(src_path, OpenFlags::READ).await?;
        let mut dst = dst
            .open(
                dst_path,
                OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::TRUNCATE,
            )
            .await?;
        tokio::io::copy(&mut src, &mut dst).await?;
    }
    Ok(())
}
