//! 青簡登記在系統裡的語言 profile：同一個文本服務（一個 CLSID）在「中文(台灣)」下兩個——拼音的「青簡」與注音的「青簡注音」，
//! 都出繁體；使用者在輸入法列表裡選的是哪個，決定拼音側按配置的方案還是注音（Server 側見 `dispatch/session` 的 `scheme`）。
//! 早期版本還在「中文(簡體)」下登記過一個「青简」，現已不再支援：登記時順手把它從系統裡拿掉（[`LEGACY_PROFILES`]）。

use windows::core::GUID;

use super::flags::ProfileFlags;

/// 一個語言 profile 的登記資訊。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LanguageProfile {
    /// 登記在哪種語言下（LANGID）。
    pub langid: u16,

    /// profile 的 GUID，各自固定、不能互換（系統按它記使用者的輸入法列表）。
    pub guid: GUID,

    /// 輸入法列表裡顯示的名字。
    pub description: &'static str,

    /// 這個 profile 下的輸入行為。
    pub flags: ProfileFlags,
}

/// 「中文(繁體，台灣)」下的青簡，zh-TW：拼音、出繁體。
pub(crate) const PROFILE_ZH_TW: LanguageProfile = LanguageProfile {
    langid: 0x0404,
    guid: GUID::from_u128(0xebb074cd_467b_4f8b_b741_954e3fae6b93),
    description: "青簡",
    flags: ProfileFlags { zhuyin: false },
};

/// 「中文(繁體，台灣)」下的青簡注音：注音鍵盤（排列看 `[general] zhuyin_layout`）、出繁體。與微軟在同一語言下分「注音」「拼音」兩個輸入法一樣，
/// 注音使用者在列表裡直接選它，不必進設定改方案。
pub(crate) const PROFILE_ZH_TW_ZHUYIN: LanguageProfile = LanguageProfile {
    langid: 0x0404,
    guid: GUID::from_u128(0x5c2f7a16_3d8e_4b0a_9f61_2e7c4d9b8a53),
    description: "青簡注音",
    flags: ProfileFlags { zhuyin: true },
};

/// 註冊 / 反註冊時登記的全部 profile。
pub(crate) const PROFILES: [LanguageProfile; 2] = [PROFILE_ZH_TW, PROFILE_ZH_TW_ZHUYIN];

/// 早期版本登記過、現已移除的 profile：「中文(簡體，中國)」0x0804 下的「青简」。
/// 升級安裝時 DLL 重新註冊，這些只拿掉、不再登記，免得使用者的輸入法列表裡還留著一個簡體的青簡。
pub(crate) const LEGACY_PROFILES: [LanguageProfile; 1] = [LanguageProfile {
    langid: 0x0804,
    guid: GUID::from_u128(0x8119f8e0_cf81_423b_9189_c0d7374324b3),
    description: "青简",
    flags: ProfileFlags { zhuyin: false },
}];

/// 某個 profile GUID 對應的輸入行為；不認識的 GUID（不該出現，含早期的簡體 profile）按「中文(台灣)」的青簡（拼音）。
pub(crate) fn flags_of(profile: &GUID) -> ProfileFlags {
    PROFILES
        .iter()
        .find(|candidate| candidate.guid == *profile)
        .map(|candidate| candidate.flags)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{
        LEGACY_PROFILES, PROFILE_ZH_TW, PROFILE_ZH_TW_ZHUYIN, PROFILES, ProfileFlags, flags_of,
    };

    #[test]
    fn profiles_have_distinct_guids_and_all_live_under_taiwan_chinese() {
        let all: Vec<_> = PROFILES.iter().chain(LEGACY_PROFILES.iter()).collect();
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                assert_ne!(a.guid, b.guid, "{} / {}", a.description, b.description);
            }
        }
        assert_eq!(PROFILES.len(), 2);
        assert!(PROFILES.iter().all(|profile| profile.langid == 0x0404));
        assert_eq!(PROFILE_ZH_TW.langid, PROFILE_ZH_TW_ZHUYIN.langid);
        assert!(LEGACY_PROFILES.iter().all(|legacy| legacy.langid != 0x0404));
    }

    #[test]
    fn only_the_zhuyin_profile_is_zhuyin_and_unknown_guids_fall_back_to_pinyin() {
        assert_eq!(
            flags_of(&PROFILE_ZH_TW.guid),
            ProfileFlags { zhuyin: false }
        );
        assert_eq!(
            flags_of(&PROFILE_ZH_TW_ZHUYIN.guid),
            ProfileFlags { zhuyin: true }
        );
        assert_eq!(
            flags_of(&windows::core::GUID::zeroed()),
            ProfileFlags::default()
        );
        // 早期的簡體 profile 不再認：有人還選著它也按繁體拼音算
        assert_eq!(flags_of(&LEGACY_PROFILES[0].guid), ProfileFlags::default());
    }
}
