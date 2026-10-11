//! 短語的邊界規則：按整串的首尾字與成分詞的形狀篩掉「只是碰巧挨著」的組合。

/// 不能開頭的字：助詞、語氣詞。它們附在前一個詞上，以它們開頭的相鄰組合（的是、了一）只是碰巧挨著。
const LEADING_CHARS: &str = "的了着得地吗吧呢啊呀哦嗯";

/// 不能結尾的字：附在後一個詞上的副詞 / 介詞 / 連詞，以及數詞（是一、有兩 只是句子的碎片）。
/// 就 / 在 不在這裡：那就、放在 是人會整塊打的。
const TRAILING_CHARS: &str = "不没很太也都又再才还把被和与及或而且但一二三四五六七八九十几两";

/// 人稱代詞：代詞 + 了 / 這 / 那（你了、你這）是碎片；單字 + 的 裡代詞的（我的）總成詞。
const PRONOUNS: &str = "我你他她它您咱";

/// 代詞後面跟這些字不成詞。
const AFTER_PRONOUN: &str = "了这那";

/// 系詞 / 介詞：它們後面跟單字（是新、在中）多半是碎片。
const COPULAS: &str = "是在有和与及或";

/// 單字 + 代詞成詞的那些單字（給我、幫你、想你、那我）；其餘單字 + 代詞（這你、天你、來你）是碎片。
const BEFORE_PRONOUN: &str = "给让帮等找叫送陪问骂爱想念疼怕怨为比跟和对被靠像信那";

/// 含這些的不是短語：笑聲與語氣詞的黏連（哈哈好的、哈哈就）。
const NOISE: &str = "哈哈";

/// 這幾個成分詞拼起來能不能當短語。`strong` 是次數遠超下限：夠常見的組合只看首尾字，
/// 形狀規則（單字 + 的、系詞 + 單字）只用來篩次數剛過線的——好的 / 真的 / 是啥 是人天天打的，多的 / 歲的 / 是新 不是。
pub fn passes(parts: &[&str], strong: bool) -> bool {
    let text: String = parts.concat();
    let (Some(first), Some(last)) = (text.chars().next(), text.chars().last()) else {
        return false;
    };
    if LEADING_CHARS.contains(first) || TRAILING_CHARS.contains(last) {
        return false;
    }
    // 疊字（哈哈哈）另算，不是短語；笑聲黏上的（哈哈好的）也不是
    if text.chars().all(|c| c == first) || text.contains(NOISE) {
        return false;
    }
    let single = |part: &str| part.chars().count() == 1;
    let [head @ .., tail] = parts else {
        return false;
    };
    let head_chars: usize = head.iter().map(|p| p.chars().count()).sum();
    // 代詞 + 了 / 這 / 那（你了、你這）是碎片
    if head_chars == 1 && PRONOUNS.contains(first) && single(tail) && AFTER_PRONOUN.contains(last) {
        return false;
    }
    // 單字 + 代詞：給我 / 幫你 成詞，這你 / 天你 / 來你 是碎片
    if head_chars == 1 && single(tail) && PRONOUNS.contains(last) && !BEFORE_PRONOUN.contains(first)
    {
        return false;
    }
    if strong {
        return true;
    }
    // 單字 + 的：代詞成詞（我的），其餘次數不夠的（歲的 / 前的）是碎片
    if *tail == "的" && head_chars == 1 && !PRONOUNS.contains(first) {
        return false;
    }
    // 是 / 在 + 單字（是新、在中）是碎片
    if parts.len() == 2 && single(parts[0]) && COPULAS.contains(first) && single(tail) {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boundary_rules() {
        assert!(passes(&["我", "的"], false));
        assert!(passes(&["都", "是"], false));
        assert!(passes(&["那", "就"], false));
        assert!(passes(&["放", "在"], false));
        assert!(passes(&["不", "知道"], false));
        assert!(passes(&["给", "我"], false));
        assert!(passes(&["自己", "的"], false));
        assert!(!passes(&["的", "是"], true));
        assert!(!passes(&["了", "一"], true));
        assert!(!passes(&["是", "一"], true));
        assert!(!passes(&["我", "很"], true));
        assert!(!passes(&["哈", "哈", "哈"], true));
        assert!(!passes(&["你", "了"], true));
        assert!(!passes(&["你", "这"], true));
        assert!(!passes(&["这", "你"], true));
        assert!(!passes(&["哈哈", "好", "的"], true));
        // 形狀規則只篩次數剛過線的：好的 常見、歲的 不常見
        assert!(passes(&["好", "的"], true));
        assert!(!passes(&["岁", "的"], false));
        assert!(passes(&["是", "啥"], true));
        assert!(!passes(&["是", "新"], false));
        assert!(!passes(&[], true));
    }
}
