//! 普通話音節表。查詢邏輯在 [`super::trie`]。

/// 無聲調的普通話合法音節表（含 ü 寫作 v / ue 兩種寫法；j / q / x / y 後的 `v` 是 `u` 的鍵盤別名，查詞前經 `canonical_syllable` 歸一）。
///
/// 只放獨立成字的音節，不含 `m` / `ng` / `hm` 這類嘆詞寫法。按聲母分組便於人工核對。
#[rustfmt::skip]
pub const SYLLABLES: &[&str] = &[
    // 零聲母
    "a", "ai", "an", "ang", "ao", "e", "ei", "en", "eng", "er", "o", "ou",
    "yi", "ya", "yao", "ye", "you", "yan", "yin", "yang", "ying", "yong", "yu", "yv", "yue", "yuan", "yun", "yo",
    "wu", "wa", "wo", "wai", "wei", "wan", "wen", "wang", "weng",
    // b p m f
    "ba", "bo", "bai", "bei", "bao", "ban", "ben", "bang", "beng", "bi", "bie", "biao", "bian", "bin", "bing", "bu",
    "pa", "po", "pai", "pei", "pao", "pou", "pan", "pen", "pang", "peng", "pi", "pie", "piao", "pian", "pin", "ping", "pu",
    "ma", "mo", "me", "mai", "mei", "mao", "mou", "man", "men", "mang", "meng", "mi", "mie", "miao", "miu", "mian", "min", "ming", "mu",
    "fa", "fo", "fei", "fou", "fan", "fen", "fang", "feng", "fu",
    // d t n l
    "da", "de", "dai", "dei", "dao", "dou", "dan", "den", "dang", "deng", "dong", "di", "dia", "die", "diao", "diu", "dian", "ding", "du", "duo", "dui", "duan", "dun",
    "ta", "te", "tai", "tao", "tou", "tan", "tang", "teng", "tong", "ti", "tie", "tiao", "tian", "ting", "tu", "tuo", "tui", "tuan", "tun",
    "na", "ne", "nai", "nei", "nao", "nou", "nan", "nen", "nang", "neng", "nong", "ni", "nie", "niao", "niu", "nian", "nin", "niang", "ning", "nu", "nuo", "nuan", "nun", "nv", "nve", "nue",
    "la", "le", "lai", "lei", "lao", "lou", "lan", "lang", "leng", "long", "li", "lia", "lie", "liao", "liu", "lian", "lin", "liang", "ling", "lu", "luo", "luan", "lun", "lv", "lve", "lue", "lo",
    // g k h
    "ga", "ge", "gai", "gei", "gao", "gou", "gan", "gen", "gang", "geng", "gong", "gu", "gua", "guo", "guai", "gui", "guan", "gun", "guang",
    "ka", "ke", "kai", "kei", "kao", "kou", "kan", "ken", "kang", "keng", "kong", "ku", "kua", "kuo", "kuai", "kui", "kuan", "kun", "kuang",
    "ha", "he", "hai", "hei", "hao", "hou", "han", "hen", "hang", "heng", "hong", "hu", "hua", "huo", "huai", "hui", "huan", "hun", "huang",
    // j q x
    "ji", "jia", "jie", "jiao", "jiu", "jian", "jin", "jiang", "jing", "jiong", "ju", "jv", "jue", "juan", "jun",
    "qi", "qia", "qie", "qiao", "qiu", "qian", "qin", "qiang", "qing", "qiong", "qu", "qv", "que", "quan", "qun",
    "xi", "xia", "xie", "xiao", "xiu", "xian", "xin", "xiang", "xing", "xiong", "xu", "xv", "xue", "xuan", "xun",
    // zh ch sh r
    "zha", "zhe", "zhi", "zhai", "zhei", "zhao", "zhou", "zhan", "zhen", "zhang", "zheng", "zhong", "zhu", "zhua", "zhuo", "zhuai", "zhui", "zhuan", "zhun", "zhuang",
    "cha", "che", "chi", "chai", "chao", "chou", "chan", "chen", "chang", "cheng", "chong", "chu", "chua", "chuo", "chuai", "chui", "chuan", "chun", "chuang",
    "sha", "she", "shi", "shai", "shei", "shao", "shou", "shan", "shen", "shang", "sheng", "shu", "shua", "shuo", "shuai", "shui", "shuan", "shun", "shuang",
    "re", "ri", "rao", "rou", "ran", "ren", "rang", "reng", "rong", "ru", "rua", "ruo", "rui", "ruan", "run",
    // z c s
    "za", "ze", "zi", "zai", "zei", "zao", "zou", "zan", "zen", "zang", "zeng", "zong", "zu", "zuo", "zui", "zuan", "zun",
    "ca", "ce", "ci", "cai", "cao", "cou", "can", "cen", "cang", "ceng", "cong", "cu", "cuo", "cui", "cuan", "cun",
    "sa", "se", "si", "sai", "sao", "sou", "san", "sen", "sang", "seng", "song", "su", "suo", "sui", "suan", "sun",
];

/// 音節最長 6 個字母（zhuang / chuang / shuang）。
pub const MAX_SYLLABLE_LEN: usize = 6;

/// 簡拼允許單獨出現的聲母。`y` / `w` 按拼音書寫習慣也算。
pub const INITIALS: &[&str] = &[
    "b", "p", "m", "f", "d", "t", "n", "l", "g", "k", "h", "j", "q", "x", "zh", "ch", "sh", "r",
    "z", "c", "s", "y", "w",
];

/// `rest` 開頭能當聲母的長度：`zh` 開頭返回 `[1, 2]`（`z` 與 `zh` 都可能），`k` 返回 `[1]`，母音開頭為空。
pub fn initial_lengths(rest: &str) -> impl Iterator<Item = usize> + '_ {
    INITIALS
        .iter()
        .filter(|initial| rest.starts_with(*initial))
        .map(|initial| initial.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_has_no_duplicates_and_respects_max_len() {
        let mut sorted = SYLLABLES.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), SYLLABLES.len());
        assert!(SYLLABLES.iter().all(|s| s.len() <= MAX_SYLLABLE_LEN));
        assert!(
            SYLLABLES
                .iter()
                .all(|s| s.bytes().all(|b| b.is_ascii_lowercase()))
        );
    }
}
