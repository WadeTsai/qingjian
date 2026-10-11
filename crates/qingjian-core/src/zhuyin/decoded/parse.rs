//! 按既有注音規則解碼，同時對映輸入游標，不改變單元或尾串的順序。
use super::{cursor::Cursor, result::Decoded, unit::Unit};
use crate::zhuyin::{layout::map_key, syllable::ZhuyinSyllable};

pub fn decode(input: &str) -> Decoded {
    decode_at(input, input.len()).0
}

pub(crate) fn decode_at(input: &str, cursor: usize) -> (Decoded, usize) {
    let mut units = Vec::new();
    let mut current = ZhuyinSyllable::new();
    let mut tail = String::new();
    let mut mapping = Cursor::new(cursor);
    let mut start = 0;

    for (offset, c) in input.char_indices() {
        if c == '\'' {
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

        if let Some(comp) = map_key(c) {
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
