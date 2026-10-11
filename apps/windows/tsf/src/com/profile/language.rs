//! 青簡登記在系統裡的語言 profile：同一個文本服務（一個 CLSID）在「中文(簡體)」下一個、「中文(台灣)」下兩個
//!（拼音的「青簡」與大千注音的「青簡注音」），使用者在輸入法列表裡選的是哪個，決定候選預設出簡體還是繁體、
//! 拼音側按配置的方案還是注音（Server 側見 `dispatch/session` 的 `traditional_output` / `scheme`）。

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

/// 「中文(簡體，中國)」下的青簡，zh-CN。
pub(crate) const PROFILE_ZH_CN: LanguageProfile = LanguageProfile {
    langid: 0x0804,
    guid: GUID::from_u128(0x8119f8e0_cf81_423b_9189_c0d7374324b3),
    description: "青简",
    flags: ProfileFlags {
        traditional: false,
        zhuyin: false,
    },
};

/// 「中文(繁體，台灣)」下的青簡，zh-TW。裝在繁體中文系統上的使用者在這個語言下才找得到青簡。
pub(crate) const PROFILE_ZH_TW: LanguageProfile = LanguageProfile {
    langid: 0x0404,
    guid: GUID::from_u128(0xebb074cd_467b_4f8b_b741_954e3fae6b93),
    description: "青簡",
    flags: ProfileFlags {
        traditional: true,
        zhuyin: false,
    },
};

/// 「中文(繁體，台灣)」下的青簡注音：注音鍵盤（排列看 `[general] zhuyin_layout`）、出繁體。與微軟在同一語言下分「注音」「拼音」兩個輸入法一樣，
/// 注音使用者在列表裡直接選它，不必進設定改方案。
pub(crate) const PROFILE_ZH_TW_ZHUYIN: LanguageProfile = LanguageProfile {
    langid: 0x0404,
    guid: GUID::from_u128(0x5c2f7a16_3d8e_4b0a_9f61_2e7c4d9b8a53),
    description: "青簡注音",
    flags: ProfileFlags {
        traditional: true,
        zhuyin: true,
    },
};

/// 註冊 / 反註冊時登記的全部 profile。
pub(crate) const PROFILES: [LanguageProfile; 3] =
    [PROFILE_ZH_CN, PROFILE_ZH_TW, PROFILE_ZH_TW_ZHUYIN];

/// 某個 profile GUID 對應的輸入行為；不認識的 GUID（不該出現）按「中文(簡體)」的青簡。
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
        PROFILE_ZH_CN, PROFILE_ZH_TW, PROFILE_ZH_TW_ZHUYIN, PROFILES, ProfileFlags, flags_of,
    };

    #[test]
    fn profiles_have_distinct_guids() {
        for (i, a) in PROFILES.iter().enumerate() {
            for b in &PROFILES[i + 1..] {
                assert_ne!(a.guid, b.guid, "{} / {}", a.description, b.description);
            }
        }
        assert_ne!(PROFILE_ZH_CN.langid, PROFILE_ZH_TW.langid);
        assert_eq!(PROFILE_ZH_TW.langid, PROFILE_ZH_TW_ZHUYIN.langid);
        assert_eq!(PROFILES.len(), 3);
    }

    #[test]
    fn only_the_taiwan_profiles_are_traditional_and_only_one_is_zhuyin() {
        assert_eq!(flags_of(&PROFILE_ZH_CN.guid), ProfileFlags::default());
        assert_eq!(
            flags_of(&PROFILE_ZH_TW.guid),
            ProfileFlags {
                traditional: true,
                zhuyin: false
            }
        );
        assert_eq!(
            flags_of(&PROFILE_ZH_TW_ZHUYIN.guid),
            ProfileFlags {
                traditional: true,
                zhuyin: true
            }
        );
        assert_eq!(
            flags_of(&windows::core::GUID::zeroed()),
            ProfileFlags::default()
        );
    }
}
