//! 部件模式在筆畫序列裡的匹配位置。

/// 模式在序列裡的匹配位置。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    /// 序列裡任意一處（取最左一處）。
    Any,

    /// 只在序列開頭。
    Prefix,

    /// 只在序列結尾。
    Suffix,

    /// 開頭或結尾，先看結尾。
    Both,
}

impl Anchor {
    /// 解析覆蓋表裡寫的錨點名，認不出返回 `None`。
    pub(super) fn parse(text: &str) -> Option<Self> {
        match text {
            "any" => Some(Self::Any),
            "prefix" => Some(Self::Prefix),
            "suffix" => Some(Self::Suffix),
            "both" => Some(Self::Both),
            _ => None,
        }
    }

    /// 按位置把序列裡的一處 `from` 換成 `to`，沒命中返回 `None`。
    pub(super) fn rewrite(self, seq: &str, from: &str, to: &str) -> Option<String> {
        match self {
            Self::Any => {
                let at = seq.find(from)?;
                Some(format!("{}{}{}", &seq[..at], to, &seq[at + from.len()..]))
            }
            Self::Prefix => seq.strip_prefix(from).map(|rest| format!("{to}{rest}")),
            Self::Suffix => seq.strip_suffix(from).map(|head| format!("{head}{to}")),
            Self::Both => Self::Suffix
                .rewrite(seq, from, to)
                .or_else(|| Self::Prefix.rewrite(seq, from, to)),
        }
    }
}
