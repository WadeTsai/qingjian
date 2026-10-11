//! 「統計」頁：輸入量（今天 / 7 天 / 累計）、折成幾本書、學習語言的詞彙與等級分佈。
//! 直讀 `%APPDATA%\Qingjian` 下的 `usage.tsv` / `user-vocab.tsv`，不經 Server；開啟這頁時讀一次。

use jiff::Zoned;
use qingjian_core::{Language, Usage, UsageSummary, VocabularySummary, book_scale};
use qingjian_learning::{UsageStats, VocabularyBook};
use qingjian_translate::LevelTable;
use windows_reactor::*;

use crate::panel::Settings;
use crate::panel::controls::{note, page, repo_resource};

const COLUMNS: [&str; 4] = ["漢字", "中文詞", "英文詞", "上屏次數"];

fn language_name(language: Language) -> &'static str {
    match language {
        Language::English => "英語",
        Language::Japanese => "日語",
        Language::Spanish => "西班牙語",
        Language::Vietnamese => "越南語",
        Language::Chinese => "中文",
    }
}

fn cell(text: impl Into<String>, width: f64) -> View {
    TextBlock::new().text(text).width(width).into()
}

/// 行首標籤 + 四個數字。
fn table_row(label: &str, values: [String; 4], strong: bool) -> View {
    let label_block = TextBlock::new()
        .text(label)
        .width(110.0)
        .font_weight(if strong {
            FontWeight::SEMI_BOLD
        } else {
            FontWeight::NORMAL
        });
    let cells = [
        label_block.into(),
        cell(values[0].clone(), 90.0),
        cell(values[1].clone(), 90.0),
        cell(values[2].clone(), 90.0),
        cell(values[3].clone(), 90.0),
    ];
    StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(8.0)
        .children(cells)
}

fn columns(usage: &Usage) -> [String; 4] {
    [
        group_digits(usage.hanzi),
        group_digits(usage.words),
        group_digits(usage.english_words),
        group_digits(usage.commits),
    ]
}

pub(crate) fn view(settings: &Settings, _context: &mut ViewContext<Settings>) -> View {
    let today = Zoned::now().date();
    let general = &settings.config.general;
    // 學習語言關著就沒有詞彙可統計
    let language = (!general.learning_language_off()).then(|| {
        general
            .learning_language
            .parse::<Language>()
            .unwrap_or(Language::English)
    });
    let dir = settings.data_dir();

    let usage = UsageStats::open(dir.join("usage.tsv")).summary_on(today);
    let vocabulary = language.map(|language| {
        let mut book = VocabularyBook::open(dir.join("user-vocab.tsv"));
        if let Some(path) = repo_resource(&format!("assets/levels/levels-{}.tsv", language.code()))
            && let Ok(table) = LevelTable::from_path(path)
        {
            book = book.with_levels(language, table);
        }
        book.summary_on(language, today)
    });

    let header = table_row("", COLUMNS.map(str::to_owned), true);
    let rows = [
        header,
        table_row("今天", columns(&usage.today), false),
        table_row("最近 7 天", columns(&usage.week), false),
        table_row("累計", columns(&usage.total), false),
    ];

    let body = StackPanel::new().spacing(12.0).children([
        StackPanel::new().spacing(6.0).children(rows),
        TextBlock::new()
            .text(scale_line(usage.total.hanzi))
            .font_weight(FontWeight::SEMI_BOLD)
            .into(),
        note(&since_line(&usage)),
        note("數的是上屏的文字：選一個詞算一箇中文詞，整句按詞切開數；英文候選、回車原樣上屏的英文詞與英文譯詞算英文詞。只在這臺電腦上數，與輸入日誌無關。"),
        TextBlock::new()
            .text(format!(
                "詞彙（{}）",
                language.map_or("學習語言已關", language_name)
            ))
            .font_weight(FontWeight::SEMI_BOLD)
            .into(),
        match &vocabulary {
            Some(vocabulary) => StackPanel::new()
                .spacing(12.0)
                .children([note(&vocabulary_line(vocabulary)), level_block(vocabulary)]),
            None => note("學習語言已關，不統計詞彙。"),
        },
    ]);
    page("統計", body)
}

/// 有等級表才顯示，一級一行。
fn level_block(vocabulary: &VocabularySummary) -> View {
    if vocabulary.levels.is_empty() {
        return note("裝了詞彙等級表後，這裡按 CEFR / JLPT 等級列詞彙分佈。");
    }
    let mut rows: Vec<View> = Vec::with_capacity(vocabulary.levels.len() + 1);
    rows.push(table_row(
        "等級",
        ["詞表", "見過", "看熟", "上屏過"].map(str::to_owned),
        true,
    ));
    for level in &vocabulary.levels {
        rows.push(table_row(
            &level.name,
            [
                group_digits(level.total),
                group_digits(level.seen),
                group_digits(level.familiar),
                group_digits(level.committed),
            ],
            false,
        ));
    }
    StackPanel::new().spacing(6.0).keyed_children(
        rows.into_iter()
            .enumerate()
            .map(|(index, row)| KeyedView::new(index.to_string(), row)),
    )
}

fn vocabulary_line(summary: &VocabularySummary) -> String {
    if summary.seen == 0 {
        return "還沒見過譯詞：打中文時候選右側的譯詞就是詞彙的來源。".to_owned();
    }
    format!(
        "見過 {} 個詞，看熟 {} 個；上屏過 {} 個，直接打出過 {} 個；本週新見 {} 個。",
        group_digits(summary.seen),
        group_digits(summary.familiar),
        group_digits(summary.committed),
        group_digits(summary.used),
        group_digits(summary.new_this_week),
    )
}

fn since_line(summary: &UsageSummary) -> String {
    match &summary.since {
        Some(date) => format!("自 {date} 起，有輸入的天數 {}。", summary.days),
        None => "還沒有記錄，打幾個字再來看。".to_owned(),
    }
}

/// 「累計輸入 20.4 萬字，約等於 1.7 本《活著》（約 12 萬字）。」
fn scale_line(hanzi: u64) -> String {
    if hanzi == 0 {
        return "累計輸入 0 字。".to_owned();
    }
    let (book, ratio) = book_scale(hanzi);
    format!(
        "累計輸入 {}，約等於 {} 本《{}》（約 {}）。",
        hanzi_text(hanzi),
        format_ratio(ratio),
        book.title,
        hanzi_text(book.hanzi),
    )
}

/// 千位分隔。
fn group_digits(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// 萬以下整數，萬 / 億以上一位小數。
fn hanzi_text(value: u64) -> String {
    const WAN: f64 = 10_000.0;
    const YI: f64 = 100_000_000.0;
    let value_f = value as f64;
    if value_f >= YI {
        format!("{} 億字", trim_decimal(value_f / YI))
    } else if value_f >= WAN {
        format!("{} 萬字", trim_decimal(value_f / WAN))
    } else {
        format!("{} 字", group_digits(value))
    }
}

/// 一位小數，`.0` 去掉。
fn trim_decimal(value: f64) -> String {
    let text = format!("{value:.1}");
    text.strip_suffix(".0").map_or(text.clone(), str::to_owned)
}

/// 不到 10 倍留一位小數，再多取整。
fn format_ratio(ratio: f64) -> String {
    if ratio >= 10.0 {
        format!("{}", ratio.round() as u64)
    } else {
        trim_decimal(ratio)
    }
}
