mod asset;
mod fetch;
mod release;
mod signature;

pub use asset::Asset;
pub(crate) use fetch::fetch_index;
pub use release::Release;
pub use signature::{PUBLIC_KEYS, verify};

use serde::Deserialize;

use qingjian_platform::UpdateChannel;

use crate::{Available, Target, UpdateError, Version};

/// 認得的索引格式版本；發版側改了已有欄位的含義才會加一，見到別的號就不讀。
pub const SCHEMA_VERSION: u32 = 1;

/// 版本索引 `releases.json`，只取檢查更新用得著的欄位。
#[derive(Debug, Clone, Deserialize)]
pub struct Index {
    pub schema_version: u32,

    /// 從新到舊。
    pub releases: Vec<Release>,
}

impl Index {
    /// 解析已經驗過簽名的索引。
    pub fn parse(bytes: &[u8]) -> Result<Self, UpdateError> {
        let index: Self = serde_json::from_slice(bytes)?;
        if index.schema_version != SCHEMA_VERSION {
            return Err(UpdateError::UnsupportedSchema(index.schema_version));
        }
        Ok(index)
    }

    /// 比 `current` 新、渠道里看得到、有這臺機器安裝包的版本里最新的那個。
    pub fn newest(
        &self,
        current: &Version,
        target: Target,
        channel: UpdateChannel,
    ) -> Option<Available> {
        self.releases
            .iter()
            .filter(|release| channel.includes(&release.channel))
            .filter(|release| {
                release
                    .assets
                    .iter()
                    .any(|asset| asset.platform == target.platform && asset.cpu == target.cpu)
            })
            .filter_map(|release| Some((Version::parse(&release.version)?, release)))
            .filter(|(version, _)| version > current)
            .max_by(|(left, _), (right, _)| left.cmp(right))
            .map(|(_, release)| Available {
                version: release.version.clone(),
                channel: release.channel.clone(),
                date: release.date.clone(),
                notes: release.notes.clone(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAC: Target = Target {
        platform: "macos",
        cpu: "arm64",
    };
    const WINDOWS: Target = Target {
        platform: "windows",
        cpu: "x86_64",
    };

    fn index() -> Index {
        Index::parse(
            br#"{
              "schema_version": 1,
              "releases": [
                {"version": "0.1.5-beta.1", "channel": "beta", "assets": [{"platform": "macos", "cpu": "arm64"}]},
                {"version": "0.1.4", "channel": "stable", "date": "2026-10-01", "notes": ["a"],
                 "assets": [{"platform": "macos", "cpu": "arm64"}, {"platform": "macos", "cpu": "x86_64"}]},
                {"version": "0.1.4", "channel": "stable", "assets": [{"platform": "windows", "cpu": "x86_64"}]},
                {"version": "0.1.3", "channel": "stable", "assets": [{"platform": "macos", "cpu": "arm64"}]}
              ]
            }"#,
        )
        .unwrap()
    }

    fn newest(current: &str, target: Target, channel: UpdateChannel) -> Option<String> {
        index()
            .newest(&Version::parse(current).unwrap(), target, channel)
            .map(|available| available.version)
    }

    #[test]
    fn stable_channel_skips_prereleases() {
        assert_eq!(
            newest("0.1.3", MAC, UpdateChannel::Stable).as_deref(),
            Some("0.1.4")
        );
        assert_eq!(newest("0.1.4", MAC, UpdateChannel::Stable), None);
    }

    #[test]
    fn beta_channel_takes_the_newest_of_both() {
        assert_eq!(
            newest("0.1.3", MAC, UpdateChannel::Beta).as_deref(),
            Some("0.1.5-beta.1")
        );
        // 測試版使用者在正式版出來後升到正式版
        assert_eq!(
            newest("0.1.4-beta.2", WINDOWS, UpdateChannel::Beta).as_deref(),
            Some("0.1.4")
        );
    }

    #[test]
    fn only_releases_with_a_package_for_this_machine_count() {
        // 0.1.5-beta.1 只發了 macOS
        assert_eq!(newest("0.1.4", WINDOWS, UpdateChannel::Beta), None);
    }

    #[test]
    fn unknown_schema_is_rejected() {
        let error = Index::parse(br#"{"schema_version": 2, "releases": []}"#).unwrap_err();
        assert!(matches!(error, UpdateError::UnsupportedSchema(2)));
    }
}
