//! 樣例幀：離線預覽（`examples/preview.rs`）與快照測試（`tests/snapshots.rs`）共用。
//! 前幾個與真機上敲同樣拼音看到的候選窗對照，內容要和引擎當時給的一致（人工從截圖抄）。

#![allow(dead_code)]

pub use qingjian_render::sample::{annotated, nihao, nihao_with_sentence};
use qingjian_render::{
    Frame, Layout, Mode, Preedit, PreeditSegment, PreeditStyle, Row, StatusCell, Tone,
};

/// 候選窗樣例：名字、幀、排布。
pub fn candidate_scenes() -> Vec<(&'static str, Frame, Layout)> {
    vec![
        ("nihao-vertical", nihao(), Layout::Vertical),
        (
            "nihao-horizontal",
            nihao_with_sentence(),
            Layout::Horizontal,
        ),
        ("cloud-vertical", cloud(), Layout::Vertical),
        ("cloud-horizontal", cloud(), Layout::Horizontal),
        ("corrected-vertical", corrected_japanese(), Layout::Vertical),
        (
            "corrected-horizontal",
            corrected_japanese(),
            Layout::Horizontal,
        ),
        ("probe", probe(), Layout::Vertical),
        ("preedit-only", preedit_only(), Layout::Vertical),
        ("short-vertical", short(), Layout::Vertical),
        ("aux-code-vertical", aux_code(), Layout::Vertical),
        ("aux-code-horizontal", aux_code(), Layout::Horizontal),
    ]
}

/// Windows 懸浮狀態條的三格。
pub fn status_cells() -> Vec<StatusCell> {
    vec![
        StatusCell::text("中 · 小鹤", true),
        StatusCell::text("，。", true),
        StatusCell::Gear,
    ]
}

/// 帶雲聯想的一頁：整句補全與雲端詞各帶雲朵。
pub fn cloud() -> Frame {
    let mut frame = nihao();
    frame.rows.truncate(3);
    frame
        .rows
        .push(annotated(3, "你好吗", &[("hello?", Tone::Gloss)], true));
    frame.sentence = Some("你好，世界".to_owned());
    frame.footer = Some("1/3".to_owned());
    frame
}

/// 糾錯後的拼音行（刪除線 + 淡色剩餘）加日文譯詞（漢字注假名）。
pub fn corrected_japanese() -> Frame {
    Frame {
        preedit: Some(Preedit {
            segments: vec![
                PreeditSegment {
                    text: "kai".to_owned(),
                    style: PreeditStyle::Typed,
                },
                PreeditSegment {
                    text: "fs".to_owned(),
                    style: PreeditStyle::Struck,
                },
                PreeditSegment {
                    text: "'fa".to_owned(),
                    style: PreeditStyle::Rest,
                },
            ],
            cursor: 5,
        }),
        rows: vec![
            annotated(
                0,
                "开发",
                &[
                    ("v. ", Tone::Pos),
                    ("開発", Tone::Gloss),
                    ("(かいはつ)", Tone::Faint),
                    ("する", Tone::Gloss),
                ],
                false,
            ),
            annotated(
                1,
                "开",
                &[
                    ("v. ", Tone::Pos),
                    ("開", Tone::Fresh),
                    ("(ひら)", Tone::Faint),
                    ("く", Tone::Fresh),
                ],
                false,
            ),
        ],
        highlighted: Some(1),
        footer: None,
        sentence: None,
        status: Some("已删除「开放」".to_owned()),
        mode: Mode::default(),
        columns: 0,
        column_ems: Vec::new(),
    }
}

/// 四條驗收用的一行：漢字（zh 字形）、英文、彩色 emoji、日文假名與漢字。
pub fn probe() -> Frame {
    Frame {
        preedit: None,
        rows: vec![Row::plain(0, "青简 hello 🙂 日本語 骨直曜")],
        highlighted: None,
        footer: None,
        sentence: None,
        status: None,
        mode: Mode::default(),
        columns: 0,
        column_ems: Vec::new(),
    }
}

/// 輔碼態：拼音行末尾是觸發鍵與帶下劃線的碼段，候選詞後面跟著命中的碼。
pub fn aux_code() -> Frame {
    let mut rows = vec![
        annotated(
            0,
            "开发",
            &[("v. ", Tone::Pos), ("develop", Tone::Gloss)],
            false,
        ),
        annotated(
            1,
            "开饭",
            &[("v. ", Tone::Pos), ("serve meal", Tone::Gloss)],
            false,
        ),
        annotated(2, "咖啡", &[], false),
    ];
    for (row, code) in rows.iter_mut().zip(["[kf]", "[kfu]", "[kfe]"]) {
        row.code = Some(code.to_owned());
    }
    Frame {
        preedit: Some(Preedit {
            segments: vec![
                PreeditSegment {
                    text: "kai'fa;".into(),
                    style: PreeditStyle::Typed,
                },
                PreeditSegment {
                    text: "kf".into(),
                    style: PreeditStyle::AuxCode,
                },
            ],
            cursor: 9,
        }),
        rows,
        highlighted: Some(0),
        footer: Some("1/1".into()),
        sentence: None,
        status: None,
        mode: Mode::default(),
        columns: 0,
        column_ems: Vec::new(),
    }
}

/// 組句中還沒有候選：只有拼音行。
pub fn preedit_only() -> Frame {
    Frame {
        preedit: Some(Preedit::plain("zhong'wen'shu'ru", 8)),
        ..Frame::default()
    }
}

/// 候選都很短、沒有譯文、不高亮：豎排視窗要撐到最小寬度。
pub fn short() -> Frame {
    Frame {
        preedit: Some(Preedit::plain("a", 1)),
        rows: vec![
            Row::plain(0, "啊"),
            Row::plain(1, "阿"),
            Row::plain(2, "吖"),
        ],
        highlighted: None,
        footer: None,
        sentence: None,
        status: None,
        mode: Mode::default(),
        columns: 0,
        column_ems: Vec::new(),
    }
}
