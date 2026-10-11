//! 樣例幀：主題預覽圖（[`crate::preview`]）、離線預覽（`examples/preview.rs`）與快照測試共用。
//! 與真機上敲同樣拼音看到的候選窗對照，內容要和引擎當時給的一致（人工從截圖抄）。

use crate::{Frame, Mode, Preedit, Row, Tone};

/// 真機敲「nihao」看到的第一頁（2026-09-13 從截圖抄），拼音行帶游標、譯文、生詞橙色、頁碼。
pub fn nihao() -> Frame {
    Frame {
        preedit: Some(Preedit::plain("ni'hao", 6)),
        rows: vec![
            annotated(
                0,
                "你好",
                &[
                    ("int. ", Tone::Pos),
                    ("hello", Tone::Gloss),
                    (" · ", Tone::Separator),
                    ("int. ", Tone::Pos),
                    ("hi", Tone::Gloss),
                ],
                false,
            ),
            annotated(1, "👋", &[("你好", Tone::Gloss)], false),
            annotated(2, "你好好", &[], false),
            annotated(
                3,
                "你好像",
                &[("phr. ", Tone::Pos), ("you seem", Tone::Fresh)],
                false,
            ),
            annotated(4, "你好久", &[], false),
            annotated(5, "你好看", &[], false),
            annotated(6, "你哈", &[], false),
            annotated(
                7,
                "你换",
                &[
                    ("phr. ", Tone::Pos),
                    ("you change", Tone::Fresh),
                    (" · ", Tone::Separator),
                    ("phr. ", Tone::Pos),
                    ("you swap", Tone::Fresh),
                ],
                false,
            ),
            annotated(
                8,
                "你会",
                &[("phr. ", Tone::Pos), ("you will", Tone::Fresh)],
                false,
            ),
        ],
        highlighted: Some(0),
        footer: Some("1/6".to_owned()),
        sentence: None,
        status: None,
        mode: Mode::default(),
        columns: 0,
        column_ems: Vec::new(),
    }
}

/// 橫排真機截圖那一次雲端整句到了：拼音行右側帶雲朵的整句補全。
pub fn nihao_with_sentence() -> Frame {
    let mut frame = nihao();
    frame.sentence = Some("你好，很高兴认识你！".to_owned());
    frame
}

/// 一行候選：序號從 0 數，`annotation` 是譯文各段與色調。
pub fn annotated(index: usize, text: &str, annotation: &[(&str, Tone)], cloud: bool) -> Row {
    Row {
        index: (index + 1).to_string(),
        text: text.to_owned(),
        annotation: annotation
            .iter()
            .map(|(s, tone)| ((*s).to_owned(), *tone))
            .collect(),
        cloud,
        code: None,
    }
}
