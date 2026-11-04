use super::super::{compatibility, install::PackRegistry, settings, update};
use crate::server::config::UpdateChannels;
use crate::server::core::update::compare_versions;

#[derive(Clone, Copy)]
pub(super) enum InstallPolicy {
    Manual,
    Automatic(UpdateChannels),
    Required { automatic: bool },
}

impl InstallPolicy {
    pub(super) fn signed(self) -> bool {
        !matches!(self, Self::Manual)
    }

    pub(super) fn automatic(self) -> bool {
        matches!(self, Self::Automatic(_) | Self::Required { automatic: true })
    }

    pub(super) fn validate(self, id: &str, asset: &update::PackAsset) -> Result<(), String> {
        let installed = PackRegistry::load().packs.into_iter().find(|pack| pack.id == id);
        if installed.as_ref().is_some_and(|pack| {
            compare_versions(&asset.version, &pack.version) == Some(-1)
        }) {
            return Err("已安装的插件版本更高，本次下载未安装，避免降级".to_string());
        }
        match self {
            Self::Automatic(channels) => {
                let settings = settings::current();
                let allowed = match installed {
                    Some(pack) => settings.auto_update && pack.enabled && pack.signed
                        && compare_versions(&asset.version, &pack.version) == Some(1),
                    None => settings.auto_install_official,
                };
                if !allowed || !settings.auto_check || settings.channels() != channels {
                    return Err("自动安装已关闭或通道已改变，本次下载未安装".to_string());
                }
            }
            Self::Required { automatic } => {
                let pack = installed.ok_or_else(|| "插件已卸载，本次补齐未安装".to_string())?;
                if compatibility::version_error(id, &pack.version).is_none() {
                    return Err("插件已满足最低版本要求，本次下载未安装".to_string());
                }
                if automatic && !pack.signed {
                    return Err("插件已被替换为未签名包，本次补齐未安装".to_string());
                }
                if compatibility::version_error(id, &asset.version).is_some()
                    || compare_versions(&asset.version, &pack.version) != Some(1)
                {
                    return Err("所选插件未满足最低版本要求或会导致降级，已拒绝安装".to_string());
                }
            }
            Self::Manual => {}
        }
        Ok(())
    }
}
