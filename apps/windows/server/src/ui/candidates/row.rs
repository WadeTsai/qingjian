//! 候選視窗的一行：[`Candidate`] → 渲染器的 [`Row`]（序號、候選詞、annotation 片段），與 macOS 端 `candidates/row.rs` 一致。
//! GDI 畫法也用同一個型別。

use qingjian_core::{Candidate, CandidateKind};
use qingjian_render::{Row, Tone};

/// `position` 是頁內下標（從 0 起）。`show_code` 是 `[general] aux_code_show`：
/// 開啟且候選帶碼時，碼用方括號括起來緊跟在候選詞後面（`鶴[rbm]`），不進 annotation。
pub(crate) fn from_candidate(position: usize, candidate: &Candidate, show_code: bool) -> Row {
    let code = candidate
        .aux_code
        .as_ref()
        .filter(|_| show_code)
        .map(|code| format!("[{code}]"));
    let mut annotation = Vec::new();
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
    Row {
        index: (position + 1).to_string(),
        text: candidate.text.clone(),
        code,
        annotation,
        cloud: candidate.kind == CandidateKind::Cloud,
    }
}
