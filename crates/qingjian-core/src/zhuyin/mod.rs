//! 注音（Bopomofo）解碼與鍵盤排列對映：按鍵 → 注音符號 → 拼音。排列（大千 / 倚天）見 [`ZhuyinLayout`]。
pub mod decoded;
pub mod layout;
pub mod syllable;

pub(crate) use decoded::decode_at;
pub use decoded::{Decoded, decode};
pub use layout::{Component, ZhuyinLayout};
pub use syllable::ZhuyinSyllable;

#[cfg(test)]
mod tests {
    use super::{ZhuyinLayout, decode};

    #[test]
    fn test_decode_bubu() {
        let d = decode(ZhuyinLayout::Dachen, "1j41j4");
        assert_eq!(d.pinyin(), "bu'bu");
        assert_eq!(d.marked(), "ㄅㄨˋㄅㄨˋ");
        assert_eq!(d.units().len(), 2);
        assert!(d.is_complete());
    }

    #[test]
    fn test_decode_incomplete() {
        let d = decode(ZhuyinLayout::Dachen, "su"); // ㄋㄧ
        assert_eq!(d.pinyin(), "ni");
        assert_eq!(d.marked(), "ㄋㄧ");
        assert!(!d.is_complete());
    }

    #[test]
    fn test_decode_with_space_as_tone1() {
        let d = decode(ZhuyinLayout::Dachen, "su "); // ㄋㄧ (第一聲)
        assert_eq!(d.pinyin(), "ni");
        assert_eq!(d.marked(), "ㄋㄧ"); // 第一聲不顯示標記
        assert!(d.is_complete());
    }

    #[test]
    fn test_decode_zhi() {
        let d = decode(ZhuyinLayout::Dachen, "5 "); // ㄓ (第一聲)
        assert_eq!(d.pinyin(), "zhi");
        assert_eq!(d.marked(), "ㄓ");
        assert!(d.is_complete());

        let d2 = decode(ZhuyinLayout::Dachen, "5"); // 只有聲母
        assert_eq!(d2.pinyin(), "zh"); // 未完成時作為字首
        assert!(!d2.is_complete());
    }

    /// 倚天排列解出同樣的符號與拼音：ㄅ=b ㄨ=x ˋ=4。
    #[test]
    fn eten_decodes_the_same_syllables_from_its_own_keys() {
        let d = decode(ZhuyinLayout::Eten, "bx4bx4");
        assert_eq!(d.pinyin(), "bu'bu");
        assert_eq!(d.marked(), "ㄅㄨˋㄅㄨˋ");
        assert!(d.is_complete());

        // ㄋ=n ㄧ=e；大千的 s / u 在倚天是 ㄙ / ㄩ
        assert_eq!(decode(ZhuyinLayout::Eten, "ne").pinyin(), "ni");
        assert_eq!(decode(ZhuyinLayout::Eten, "su").marked(), "ㄙㄩ");
    }

    /// 倚天的 `'` 是 ㄘ，不是隔音符；`=` 是 ㄦ。
    #[test]
    fn eten_apostrophe_is_an_initial_and_equals_is_er() {
        let d = decode(ZhuyinLayout::Eten, "'a4");
        assert_eq!(d.marked(), "ㄘㄚˋ");
        assert_eq!(d.pinyin(), "ca");
        let d = decode(ZhuyinLayout::Eten, "=2");
        assert_eq!(d.marked(), "ㄦˊ");
        assert_eq!(d.pinyin(), "er");
        // 大千下 `'` 仍是隔音符，不是符號
        let d = decode(ZhuyinLayout::Dachen, "1j4'1j4");
        assert_eq!(d.pinyin(), "bu'bu");
        assert_eq!(d.units().len(), 3);
    }
}
