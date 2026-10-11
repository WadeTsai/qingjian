//! 青简登记在系统里的语言 profile：同一个文本服务（一个 CLSID）在「中文(简体)」与「中文(台灣)」下各一个，
//! 用户在哪种语言下选的青简，决定候选缺省出简体还是繁体（Server 侧见 `dispatch/session` 的 `traditional_output`）。

use windows::core::GUID;

/// 一个语言 profile 的登记信息。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LanguageProfile {
    /// 登记在哪种语言下（LANGID）。
    pub langid: u16,

    /// profile 的 GUID，两个 profile 各自固定、不能互换（系统按它记用户的输入法列表）。
    pub guid: GUID,

    /// 输入法列表里显示的名字。
    pub description: &'static str,

    /// 这个 profile 下缺省出繁体。
    pub traditional: bool,
}

/// 「中文(简体，中国)」下的青简，zh-CN。
pub(crate) const PROFILE_ZH_CN: LanguageProfile = LanguageProfile {
    langid: 0x0804,
    guid: GUID::from_u128(0x8119f8e0_cf81_423b_9189_c0d7374324b3),
    description: "青简",
    traditional: false,
};

/// 「中文(繁體，台灣)」下的青簡，zh-TW。装在繁体中文系统上的用户在这个语言下才找得到青简。
pub(crate) const PROFILE_ZH_TW: LanguageProfile = LanguageProfile {
    langid: 0x0404,
    guid: GUID::from_u128(0xebb074cd_467b_4f8b_b741_954e3fae6b93),
    description: "青簡",
    traditional: true,
};

/// 注册 / 反注册时登记的全部 profile。
pub(crate) const PROFILES: [LanguageProfile; 2] = [PROFILE_ZH_CN, PROFILE_ZH_TW];

/// 某个 profile GUID 是不是繁体的那个；不认识的 GUID（不该出现）按简体。
pub(crate) fn is_traditional(profile: &GUID) -> bool {
    PROFILES
        .iter()
        .any(|candidate| candidate.traditional && candidate.guid == *profile)
}

#[cfg(test)]
mod tests {
    use super::{PROFILES, PROFILE_ZH_CN, PROFILE_ZH_TW, is_traditional};

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
