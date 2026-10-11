//! 青簡登記在系統裡的語言 profile：同一個文本服務（一個 CLSID）在「中文(簡體)」與「中文(台灣)」下各一個，
//! 使用者在哪種語言下選的青簡，決定候選預設出簡體還是繁體（Server 側見 `dispatch/session` 的 `traditional_output`）。

use windows::core::GUID;

/// 一個語言 profile 的登記資訊。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LanguageProfile {
    /// 登記在哪種語言下（LANGID）。
    pub langid: u16,

    /// profile 的 GUID，兩個 profile 各自固定、不能互換（系統按它記使用者的輸入法列表）。
    pub guid: GUID,

    /// 輸入法列表裡顯示的名字。
    pub description: &'static str,

    /// 這個 profile 下預設出繁體。
    pub traditional: bool,
}

/// 「中文(簡體，中國)」下的青簡，zh-CN。
pub(crate) const PROFILE_ZH_CN: LanguageProfile = LanguageProfile {
    langid: 0x0804,
    guid: GUID::from_u128(0x8119f8e0_cf81_423b_9189_c0d7374324b3),
    description: "青简",
    traditional: false,
};

/// 「中文(繁體，台灣)」下的青簡，zh-TW。裝在繁體中文系統上的使用者在這個語言下才找得到青簡。
pub(crate) const PROFILE_ZH_TW: LanguageProfile = LanguageProfile {
    langid: 0x0404,
    guid: GUID::from_u128(0xebb074cd_467b_4f8b_b741_954e3fae6b93),
    description: "青簡",
    traditional: true,
};

/// 註冊 / 反註冊時登記的全部 profile。
pub(crate) const PROFILES: [LanguageProfile; 2] = [PROFILE_ZH_CN, PROFILE_ZH_TW];

/// 某個 profile GUID 是不是繁體的那個；不認識的 GUID（不該出現）按簡體。
pub(crate) fn is_traditional(profile: &GUID) -> bool {
    PROFILES.iter().any(|candidate| candidate.traditional && candidate.guid == *profile)
}

#[cfg(test)]
mod tests {
    use super::{PROFILE_ZH_CN, PROFILE_ZH_TW, PROFILES, is_traditional};

    #[test]
    fn the_two_profiles_differ_in_guid_and_language() {
        assert_ne!(PROFILE_ZH_CN.guid, PROFILE_ZH_TW.guid);
        assert_ne!(PROFILE_ZH_CN.langid, PROFILE_ZH_TW.langid);
        assert_eq!(PROFILES.len(), 2);
    }

    #[test]
    fn only_the_taiwan_profile_is_traditional() {
        assert!(is_traditional(&PROFILE_ZH_TW.guid));
        assert!(!is_traditional(&PROFILE_ZH_CN.guid));
        assert!(!is_traditional(&windows::core::GUID::zeroed()));
    }
}
