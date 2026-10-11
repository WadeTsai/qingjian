//! 按既有注音規則解碼，同時對映輸入游標，不改變單元或尾串的順序。按鍵按哪套排列解看 [`ZhuyinLayout`]。
use super::{cursor::Cursor, result::Decoded, unit::Unit};
use crate::zhuyin::{layout::ZhuyinLayout, syllable::ZhuyinSyllable};

pub fn decode(layout: ZhuyinLayout, input: &str) -> Decoded {
    decode_at(layout, input, input.len()).0
}

pub(crate) fn decode_at(layout: ZhuyinLayout, input: &str, cursor: usize) -> (Decoded, usize) {
    let mut units = Vec::new();
    let mut current = ZhuyinSyllable::new();
    let mut tail = String::new();
    let mut mapping = Cursor::new(layout, cursor);
    let mut start = 0;

    for (offset, c) in input.char_indices() {
        // 倚天排列裡 `'` 是 ㄘ，不再是隔音符；大千下仍是隔音符
        if c == '\'' && layout.map_key(c).is_none() {
            if !current.is_empty() {
                push_syllable(&mut units, &mut mapping, start, &current);
                current = ZhuyinSyllable::new();
            }
            mapping.unit(offset, "'", "'");
            units.push(Unit {
                pinyin: "'".to_string(),
                display: "'".to_string(),
                keys: "'".to_string(),
                complete: true,
            });
            continue;
        }

        if let Some(comp) = layout.map_key(c) {
            if current.is_empty() {
                start = offset;
            }
            if !current.push(comp, c) {
                // 不相容，將 current 推入 units，並開啟新音節
                if !current.is_empty() {
                    push_syllable(&mut units, &mut mapping, start, &current);
                }
                current = ZhuyinSyllable::new();
                start = offset;
                if !current.push(comp, c) {
                    mapping.tail(offset, c);
                    tail.push(c);
                }
            }
        } else {
            // 無法解析的字元，直接放到尾巴或當前就中斷
            if !current.is_empty() {
                push_syllable(&mut units, &mut mapping, start, &current);
                current = ZhuyinSyllable::new();
            }
            mapping.tail(offset, c);
            tail.push(c);
        }
    }

    if !current.is_empty() {
        push_syllable(&mut units, &mut mapping, start, &current);
    }

    let cursor = mapping.finish(input.len());
    (Decoded::new(units, tail), cursor)
}

fn push_syllable(
    units: &mut Vec<Unit>,
    mapping: &mut Cursor,
    start: usize,
    current: &ZhuyinSyllable,
) {
    let unit = Unit {
        pinyin: current.to_pinyin(),
        display: current.display_string(),
        keys: current.keys.clone(),
        complete: current.has_tone(),
    };
    mapping.unit(start, &unit.keys, &unit.display);
    units.push(unit);
}
