//! 候選視窗的一行：序號、候選詞、annotation 片段。只是 Core 輸出的展示形態，不含任何排序或查詞。

use qingjian_core::Candidate;

/// annotation 片段的種類，決定顏色；青簡渲染器的主題還按它挑著畫。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// 譯文。
    Gloss,

    /// 生詞的譯文（使用者還沒在候選裡見過幾輪，`Sense::fresh`），用強調色。
    Fresh,

    /// 其餘的淡色片段（假名注音），最淺。
    Faint,

    /// 詞性，與 `Faint` 同色。
    Pos,

    /// 義項之間的分隔，與 `Faint` 同色。
    Separator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// 顯示用序號文本，如 `1`。
    pub index: String,

    /// 候選詞。
    pub text: String,

    /// 右側 annotation，按順序繪製；沒有譯文時為空。
    pub annotation: Vec<(String, Tone)>,

    /// 來自雲聯想：詞前畫一個小云朵，與本地候選區分。
    pub cloud: bool,
}

impl Row {
    pub fn from_candidate(position: usize, candidate: &Candidate) -> Self {
        let mut annotation = Vec::new();
        // 讀音（問字模式答案的帶聲調拼音）放在最前
        if let Some(reading) = &candidate.reading {
            annotation.push((reading.clone(), Tone::Gloss));
        }
        if let Some(translation) = &candidate.translation {
            for (i, sense) in translation.senses().iter().enumerate() {
                if i > 0 || !annotation.is_empty() {
                    annotation.push((" · ".to_owned(), Tone::Separator));
                }
                if let Some(pos) = sense.part_of_speech {
                    annotation.push((format!("{pos} "), Tone::Pos));
                }
                // 日文譯詞按漢欄位注平假名（開発(かいはつ)する），假名淡色
                let tone = if sense.fresh {
                    Tone::Fresh
                } else {
                    Tone::Gloss
                };
                for segment in sense.furigana() {
                    annotation.push((segment.text, tone));
                    if let Some(reading) = segment.reading {
                        annotation.push((format!("({reading})"), Tone::Faint));
                    }
                }
            }
        }
        Self {
            index: (position + 1).to_string(),
            text: if matches!(candidate.kind, qingjian_core::CandidateKind::Custom(_)) {
                qingjian_core::CustomPhrase::preview(&candidate.text, 60)
            } else {
                candidate.text.clone()
            },
            annotation,
            cloud: false,
        }
    }
}
