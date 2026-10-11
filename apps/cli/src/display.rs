//! 候選與耗時的終端排版。

use std::time::{Duration, Instant};

use qingjian_core::{Candidate, Engine, Query};

/// 等聯想結果的輪詢間隔與上限。CLI 是同步工具，等一等無妨；輸入法裡是定時器輪詢、不等。
const PREDICTION_POLL: Duration = Duration::from_millis(20);
const PREDICTION_WAIT: Duration = Duration::from_secs(8);

/// 查詢並列印一段拼音的候選、譯文和耗時。返回查詢結果供後續上屏用。
///
/// 輸入裡的 `|` 表示游標位置（`ni|hao`），用來驗證游標停在中間時的候選。
pub fn show(engine: &mut Engine, input: &str, limit: usize) -> Option<Query> {
    let cursor = input.find('|');
    feed(engine, &input.replace('|', ""));
    if let Some(cursor) = cursor {
        engine.move_cursor_home();
        for _ in 0..cursor {
            engine.move_cursor_right();
        }
    }
    let mut query = match engine.query() {
        // 非同步重打分：等後臺的分回來再查一次，輸出的就是重排後的
        Ok(query) if crate::rescoring::settle(engine) => engine.query().unwrap_or(query),
        Ok(query) => query,
        Err(error) => {
            println!("  {error}");
            return None;
        }
    };
    let report = engine.annotate(&mut query.candidates);

    let segmentations: Vec<String> = query
        .segmentations
        .iter()
        .map(ToString::to_string)
        .collect();
    if query.tail.is_empty() {
        println!("  切分: {}", segmentations.join(" | "));
    } else {
        println!(
            "  切分: {}  未切分尾部: {}",
            segmentations.join(" | "),
            query.tail
        );
    }
    if let Some(correction) = &query.correction {
        println!(
            "  纠正: {} → {}",
            correction.original, correction.segmentation
        );
    }
    if query.candidates.items.is_empty() {
        println!("  （无候选）");
    }
    let width = query
        .candidates
        .items
        .iter()
        .take(limit)
        .map(|c| display_width(&c.text))
        .max()
        .unwrap_or(0);
    for (index, candidate) in query.candidates.items.iter().enumerate().take(limit) {
        println!("  {:>2}. {}", index + 1, format_candidate(candidate, width));
    }
    let hidden = query.candidates.items.len().saturating_sub(limit);
    if hidden > 0 {
        println!("      … 还有 {hidden} 个");
    }
    let t = query.timings;
    println!(
        "  parse {} · lookup {} · rank {} · translate {} ({}/{} hit) · total {}",
        fmt_duration(t.parse),
        fmt_duration(t.lookup),
        fmt_duration(t.rank),
        fmt_duration(report.elapsed),
        report.hits,
        report.total,
        fmt_duration(t.total() + report.elapsed),
    );
    show_prediction(engine, &query.candidates.items);
    Some(query)
}

/// 與殼一樣逐個喂鍵：配的觸發鍵進輔碼態、之後的 `a-z` 進碼段，其餘進拼音緩衝區。
/// 純拼音輸入下與 [`Engine::set_input`] 等價（觸發鍵要求作用域能完整切分，字母永遠不會觸發）。
pub fn feed(engine: &mut Engine, keys: &str) {
    engine.clear();
    for key in keys.chars() {
        if engine.aux_trigger(key) {
            engine.enter_aux();
        } else if engine.in_aux() {
            if !engine.push_aux_code(key) {
                engine.clear_aux();
                engine.push(key);
            }
        } else {
            engine.push(key);
        }
    }
}

/// 逐鍵模式：`kaifa` 當作 k、ka、kai…… 五次按鍵，每個字首都查一次並標註譯文，
/// 一行一鍵列印各階段耗時和首候選。這是輸入法每鍵的真實工作量（聯想不算，它在後臺執行緒）。
pub fn show_typing(engine: &mut Engine, input: &str) {
    println!(
        "  {:<16} {:>9} {:>9} {:>9} {:>9} {:>9}  首候选",
        "输入", "parse", "lookup", "rank", "translate", "total"
    );
    let mut worst = Duration::ZERO;
    let mut sum = Duration::ZERO;
    let mut keys = 0;
    for (index, _) in input
        .char_indices()
        .skip(1)
        .chain(std::iter::once((input.len(), ' ')))
    {
        let prefix = &input[..index];
        engine.set_input(prefix);
        let mut query = match engine.query() {
            Ok(query) => query,
            Err(error) => {
                println!("  {prefix:<16} {error}");
                continue;
            }
        };
        let report = engine.annotate(&mut query.candidates);
        let t = query.timings;
        let total = t.total() + report.elapsed;
        worst = worst.max(total);
        sum += total;
        keys += 1;
        let first = query
            .candidates
            .items
            .first()
            .map(|c| c.text.as_str())
            .unwrap_or("（无候选）");
        println!(
            "  {prefix:<16} {:>9} {:>9} {:>9} {:>9} {:>9}  {first}",
            fmt_duration(t.parse),
            fmt_duration(t.lookup),
            fmt_duration(t.rank),
            fmt_duration(report.elapsed),
            fmt_duration(total),
        );
    }
    if keys > 0 {
        println!(
            "  {keys} 键 · 最慢 {} · 平均 {}",
            fmt_duration(worst),
            fmt_duration(sum / keys)
        );
    }
    engine.set_input("");
}

/// 發一次聯想並等結果打印出來。沒接 Predictor 時什麼都不做。
pub fn show_prediction(engine: &mut Engine, candidates: &[Candidate]) {
    let Some(_sequence) = engine.request_prediction(None, candidates) else {
        return;
    };
    let start = Instant::now();
    while start.elapsed() < PREDICTION_WAIT {
        if let Some(prediction) = engine.poll_prediction() {
            if prediction.is_empty() {
                println!("  ☁ （无联想）");
            }
            for word in &prediction.words {
                let reading = word
                    .reading
                    .clone()
                    .unwrap_or_else(|| word.syllables.join(" "));
                println!("  ☁ {}  ({reading})", word.text);
            }
            if let Some(sentence) = &prediction.sentence {
                println!("  ☁ 整句: {sentence}");
            }
            println!("  predict {}", fmt_duration(start.elapsed()));
            return;
        }
        std::thread::sleep(PREDICTION_POLL);
    }
    println!("  ☁ （联想超时）");
}

/// 候選詞左對齊，右側是「詞性 譯文」，多條釋義用 · 分隔。
fn format_candidate(candidate: &Candidate, width: usize) -> String {
    let padding = " ".repeat(width.saturating_sub(display_width(&candidate.text)) + 2);
    let reading = candidate
        .reading
        .as_ref()
        .map(|r| format!("{r} "))
        .unwrap_or_default();
    let annotation = candidate
        .translation
        .as_ref()
        .map(|t| {
            t.senses()
                .iter()
                .map(|s| {
                    // 日文譯詞按漢欄位注平假名：開発(かいはつ)する
                    let text: String = s
                        .furigana()
                        .iter()
                        .map(|segment| match &segment.reading {
                            Some(reading) => format!("{}({reading})", segment.text),
                            None => segment.text.clone(),
                        })
                        .collect();
                    match s.part_of_speech {
                        Some(pos) => format!("{pos} {text}"),
                        None => text,
                    }
                })
                .collect::<Vec<_>>()
                .join(" · ")
        })
        .unwrap_or_default();
    // 輔碼態命中的那條碼跟在詞後面，人工核對 `kaifa;kf` 時一眼能看到篩的是哪條
    let aux = candidate
        .aux_code
        .as_ref()
        .map(|code| format!("[{code}] "))
        .unwrap_or_default();
    let marker = match candidate.kind {
        qingjian_core::CandidateKind::Chinese | qingjian_core::CandidateKind::Code => "",
        qingjian_core::CandidateKind::English => "[en] ",
        qingjian_core::CandidateKind::Cloud => "☁ ",
        qingjian_core::CandidateKind::Shortcut => "[v] ",
        qingjian_core::CandidateKind::Custom(_) => "[custom] ",
        qingjian_core::CandidateKind::Sentence => "[句] ",
        qingjian_core::CandidateKind::Generated => "[生成] ",
        qingjian_core::CandidateKind::Emoji => "",
    };
    format!(
        "{}{padding}{marker}{reading}{aux}{annotation}",
        candidate.text
    )
    .trim_end()
    .to_owned()
}

/// 終端顯示寬度：CJK 算兩格。夠 CLI 對齊用，不引入 unicode-width。
fn display_width(text: &str) -> usize {
    text.chars()
        .map(|c| if c as u32 >= 0x2E80 { 2 } else { 1 })
        .sum()
}

fn fmt_duration(d: Duration) -> String {
    let micros = d.as_micros();
    if micros >= 1000 {
        format!("{:.2}ms", d.as_secs_f64() * 1000.0)
    } else {
        format!("{micros}µs")
    }
}
