//! 振り仮名：把日文譯詞的整體假名讀音對到各段漢字上（`開発する` + `かいはつする` → 開発(かいはつ)する）。
//!
//! 譯詞裡的假名段在讀音裡原樣出現，拿它們當錨點，錨點之間的假名就是相鄰漢欄位的讀音；
//! 多種對法時取第一種可行的。對不上（讀音與寫法的假名不一致）就整體注在後面。

mod segment;

pub use segment::FuriganaSegment;

/// 把 `text` 切成漢欄位與非漢欄位，漢欄位帶上從 `reading` 裡對出來的假名。讀音與寫法一樣（純假名詞）時只有一段、不帶讀音。
pub fn furigana(text: &str, reading: &str) -> Vec<FuriganaSegment> {
    let runs = split_runs(text);
    let reading_chars: Vec<char> = reading.chars().map(to_hiragana).collect();
    if !runs.iter().any(|(_, kanji)| *kanji) {
        return vec![FuriganaSegment {
            text: text.to_owned(),
            reading: None,
        }];
    }
    let mut readings = Vec::with_capacity(runs.len());
    if align(&runs, &reading_chars, 0, &mut readings) {
        runs.iter()
            .zip(readings)
            .map(|((run, _), reading)| FuriganaSegment {
                text: run.clone(),
                reading,
            })
            .collect()
    } else {
        vec![FuriganaSegment {
            text: text.to_owned(),
            reading: Some(reading.to_owned()),
        }]
    }
}

/// 連續的漢字為一段、連續的非漢字為一段：(段, 是否漢字)。
fn split_runs(text: &str) -> Vec<(String, bool)> {
    let mut runs: Vec<(String, bool)> = Vec::new();
    for c in text.chars() {
        let kanji = is_kanji(c);
        match runs.last_mut() {
            Some((run, last_kanji)) if *last_kanji == kanji => run.push(c),
            _ => runs.push((c.to_string(), kanji)),
        }
    }
    runs
}

/// 回溯對齊：非漢欄位必須與讀音裡對應位置逐字相同（假名先統一成平假名），漢欄位取到下一個錨點之間的假名，至少一個。
fn align(
    runs: &[(String, bool)],
    reading: &[char],
    position: usize,
    out: &mut Vec<Option<String>>,
) -> bool {
    let Some(((run, kanji), rest)) = runs.split_first() else {
        return position == reading.len();
    };
    if !*kanji {
        let chars: Vec<char> = run.chars().map(to_hiragana).collect();
        if reading[position..].starts_with(&chars) {
            out.push(None);
            if align(rest, reading, position + chars.len(), out) {
                return true;
            }
            out.pop();
        }
        return false;
    }
    // 最後一段漢字吃掉剩下的全部；中間的漢欄位從短到長試，讓後面的錨點決定邊界
    if rest.is_empty() {
        if position < reading.len() {
            out.push(Some(reading[position..].iter().collect()));
            return true;
        }
        return false;
    }
    for end in position + 1..=reading.len() {
        out.push(Some(reading[position..end].iter().collect()));
        if align(rest, reading, end, out) {
            return true;
        }
        out.pop();
    }
    false
}

/// 漢字（含 々 〆）。
fn is_kanji(c: char) -> bool {
    matches!(c, '\u{4e00}'..='\u{9fff}' | '\u{3400}'..='\u{4dbf}' | '々' | '〆')
}

/// 片假名統一成平假名，長音符等原樣。
fn to_hiragana(c: char) -> char {
    match c {
        '\u{30a1}'..='\u{30f6}' => char::from_u32(c as u32 - 0x60).unwrap_or(c),
        _ => c,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(text: &str, reading: &str) -> String {
        furigana(text, reading)
            .iter()
            .map(|s| match &s.reading {
                Some(r) => format!("{}({r})", s.text),
                None => s.text.clone(),
            })
            .collect()
    }

    #[test]
    fn annotates_each_kanji_run() {
        assert_eq!(render("開発する", "かいはつする"), "開発(かいはつ)する");
        assert_eq!(
            render("食事を始める", "しょくじをはじめる"),
            "食事(しょくじ)を始(はじ)める"
        );
        assert_eq!(render("今日", "きょう"), "今日(きょう)");
        assert_eq!(render("私たち", "わたしたち"), "私(わたし)たち");
        assert_eq!(render("お茶", "おちゃ"), "お茶(ちゃ)");
        assert_eq!(render("人々", "ひとびと"), "人々(ひとびと)");
    }

    #[test]
    fn kana_words_and_mismatches() {
        // 純假名詞不注
        assert_eq!(render("こんにちは", "こんにちは"), "こんにちは");
        assert_eq!(render("ウィーチャット", "うぃーちゃっと"), "ウィーチャット");
        // 讀音與寫法裡的假名對不上：整體注在後面
        assert_eq!(render("開発する", "かいはつ"), "開発する(かいはつ)");
        assert_eq!(render("開発する", "かいはつした"), "開発する(かいはつした)");
    }
}
