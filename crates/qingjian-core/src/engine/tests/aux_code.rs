//! 輔碼態：觸發消歧、逐鍵即篩、刪碼回退、上屏收尾與 preedit 分段。

use qingjian_dictionary::AuxCodeTable;

use super::*;

/// 測試用的小碼錶：只覆蓋樣例詞庫裡 `kaifa` 能查出來的那幾條。
/// 碼長各不相同（kf / kfz / kh），重排與逐鍵收窄的期望才好寫。
fn codes() -> Arc<dyn AuxCodeLookup> {
    Arc::new(
        AuxCodeTable::from_pairs([
            ("开发".to_owned(), "kf".to_owned()),
            ("开发者".to_owned(), "kfz".to_owned()),
            ("开".to_owned(), "kh".to_owned()),
        ])
        .unwrap(),
    )
}

fn aux_engine() -> Engine {
    let mut engine = engine().with_aux_codes(vec![codes()]);
    // 測試都按「輔碼開著且有碼錶」的常規路徑來；開關本身的邊界見下面的專項測試
    engine.set_aux_enabled(true);
    engine
}

fn candidates(engine: &Engine) -> Vec<String> {
    engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .map(|c| c.text)
        .collect()
}

/// 全拼能完整切分、游標在段尾 → 進輔碼態，碼段空、候選不變，preedit 多一個 `;`。
#[test]
fn trigger_enters_aux_without_filtering() {
    let mut engine = aux_engine();
    engine.set_input("kaifa");
    let before = candidates(&engine);
    assert!(engine.aux_trigger(';'));
    engine.enter_aux();
    assert!(engine.in_aux());
    assert_eq!(engine.aux_code(), "");
    let query = engine.query().unwrap();
    assert_eq!(query.marked_text(), "kai'fa;");
    assert_eq!(query.marked_cursor(), 7);
    assert_eq!(
        query
            .marked_segments()
            .iter()
            .map(|s| (s.text.as_str(), s.kind))
            .collect::<Vec<_>>(),
        [("kai'fa", MarkedKind::Typed), (";", MarkedKind::Typed)]
    );
    let after: Vec<String> = query
        .candidates
        .items
        .iter()
        .map(|c| c.text.clone())
        .collect();
    assert_eq!(before, after);
    assert!(after.contains(&"开发者".to_owned()));
}

/// 微軟 / 搜狗雙拼裡能當韻母的 `;` 優先當韻母（`x;` = xing），不進輔碼態。
#[test]
fn semicolon_as_a_final_beats_the_trigger() {
    let mut engine = aux_engine();
    engine.set_shuangpin(Some(Scheme::Microsoft));
    engine.set_input("x");
    assert!(engine.takes_semicolon());
    assert!(!engine.aux_trigger(';'));
    engine.push(';');
    assert_eq!(engine.query().unwrap().marked_text(), "xing");
    // 音節已經打完時 `;` 不再是韻母鍵，可以觸發
    engine.set_input("ni");
    assert!(!engine.takes_semicolon());
    assert!(engine.aux_trigger(';'));
}

/// 簡拼、殘缺音節、游標停在拼音段中間都不觸發。
#[test]
fn incomplete_or_mid_cursor_pinyin_does_not_trigger() {
    let mut engine = aux_engine();
    for text in ["kf", "kaif", "kai f"] {
        engine.set_input(text);
        assert!(!engine.aux_trigger(';'), "{text}");
    }
    engine.set_input("kaifa");
    engine.move_cursor_left();
    assert!(!engine.aux_trigger(';'));
    engine.move_cursor_end();
    assert!(engine.aux_trigger(';'));
    // 觸發鍵之外的鍵不觸發
    assert!(!engine.aux_trigger(','));
}

/// 碼段每多一個字母就重篩一次（無碼詞直接隱藏，不是降權）。
/// 碼長不同的一批：`k` 三條都在，`kf` 裡完全匹配的 開發 排第一，`kfh` 篩空。
#[test]
fn each_code_letter_narrows_the_candidates() {
    let mut engine = aux_engine();
    engine.set_input("kaifa");
    engine.enter_aux();
    assert!(engine.push_aux_code('k'));
    // 都不是完全匹配，按碼長降序：kfz(3) 的 開發者 最先，剩下兩條同長保持原序
    assert_eq!(candidates(&engine), ["开发者", "开发", "开"]);
    assert!(engine.push_aux_code('f'));
    assert_eq!(engine.aux_code(), "kf");
    assert_eq!(candidates(&engine), ["开发", "开发者"]);
    let query = engine.query().unwrap();
    assert_eq!(query.marked_text(), "kai'fa;kf");
    assert_eq!(
        query
            .marked_segments()
            .iter()
            .map(|s| (s.text.as_str(), s.kind))
            .collect::<Vec<_>>(),
        [
            ("kai'fa", MarkedKind::Typed),
            (";", MarkedKind::Typed),
            ("kf", MarkedKind::AuxCode)
        ]
    );
    // 命中的那條碼隨候選一起給殼（顯示碼開關開啟時用）
    assert_eq!(query.candidates.items[0].aux_code.as_deref(), Some("kf"));
}

/// 輔碼態只出命中碼的詞：沒有碼的候選（自定義短語、快捷、整句、emoji）一律不出；
/// 篩空時候選為空、preedit 仍在（殼據此收起候選窗、只留拼音行）。
#[test]
fn candidates_without_a_code_are_hidden() {
    let mut engine = aux_engine();
    engine.set_input("kaifa");
    engine.enter_aux();
    engine.push_aux_code('k');
    engine.push_aux_code('h');
    assert_eq!(candidates(&engine), ["开"]);
    engine.push_aux_code('z');
    let query = engine.query().unwrap();
    assert!(query.candidates.items.is_empty());
    assert_eq!(query.marked_text(), "kai'fa;khz");
}

/// 首條碼的掛載看顯示開關：純拼音態且 `aux_code_show` 關（預設）時**不逐候查碼**，
/// 候選全不帶 `aux_code`（省掉單次查詢 500 條 × 表數的二分）；`set_aux_show(true)` 後回來。
#[test]
fn pure_pinyin_skips_first_code_unless_shown() {
    let mut engine = aux_engine();
    engine.set_input("kaifa");
    let query = engine.query().unwrap();
    assert!(query.candidates.items.iter().all(|c| c.aux_code.is_none()));

    engine.set_aux_show(true);
    let query = engine.query().unwrap();
    // 測試碼錶只有 開發=kf、開發者=kfz、開=kh：其餘候選一律不帶碼
    for candidate in &query.candidates.items {
        let expected = match candidate.text.as_str() {
            "开发" => Some("kf"),
            "开发者" => Some("kfz"),
            "开" => Some("kh"),
            _ => None,
        };
        assert_eq!(
            candidate.aux_code.as_deref(),
            expected,
            "{}",
            candidate.text
        );
    }
    assert!(query.candidates.items.iter().any(|c| c.text == "开发"));
}

/// 輔碼態空碼段（剛觸發 / 刪空停住）不看顯示開關：還是掛首條碼——進輔碼態看碼有引導意義，
/// 按下觸發鍵註記不消失；篩碼時仍是命中碼。
#[test]
fn empty_code_segment_keeps_the_first_code_regardless_of_show() {
    let mut engine = aux_engine();
    engine.set_input("kaifa");
    engine.enter_aux();
    let query = engine.query().unwrap();
    assert_eq!(
        query
            .candidates
            .items
            .iter()
            .find(|c| c.text == "开发")
            .unwrap()
            .aux_code
            .as_deref(),
        Some("kf")
    );
}

/// 退格刪碼段、逐鍵放寬；刪空停在輔碼態（`aux_code_keep_empty` 預設開）——
/// `;` 仍在、無碼詞全部回來，空碼段再按一次退格才退出、拼音一字不動。
#[test]
fn backspace_widens_then_stays_in_aux_mode() {
    let mut engine = aux_engine();
    engine.set_input("kaifa");
    let full = candidates(&engine);
    engine.enter_aux();
    engine.push_aux_code('k');
    engine.push_aux_code('f');
    engine.push_aux_code('h');
    assert!(candidates(&engine).is_empty());
    assert!(engine.backspace());
    assert_eq!(engine.aux_code(), "kf");
    assert_eq!(candidates(&engine), ["开发", "开发者"]);
    assert!(engine.backspace());
    assert_eq!(engine.aux_code(), "k");
    assert_eq!(candidates(&engine), ["开发者", "开发", "开"]);
    // 刪到空：停在輔碼態，`;` 仍在 preedit，候選全回來
    assert!(engine.backspace());
    assert!(engine.in_aux());
    assert_eq!(engine.aux_code(), "");
    assert_eq!(engine.query().unwrap().marked_text(), "kai'fa;");
    assert_eq!(candidates(&engine), full);
    // 再敲字母重新篩——輔碼態還活著
    assert!(engine.push_aux_code('k'));
    assert_eq!(candidates(&engine), ["开发者", "开发", "开"]);
    assert!(engine.backspace());
    assert!(engine.in_aux());
    // 空碼段再按退格：退出輔碼態，拼音一字不動
    assert!(engine.backspace());
    assert!(!engine.in_aux());
    assert_eq!(engine.query().unwrap().marked_text(), "kai'fa");
    assert_eq!(candidates(&engine), full);
    assert_eq!(engine.composition().text(), "kaifa");
}

/// `aux_code_keep_empty = false` 刪空即回拼音態（開關關掉的舊行為）。
#[test]
fn backspace_leaves_aux_mode_when_keep_empty_is_off() {
    let mut engine = aux_engine();
    engine.set_aux_keep_empty(false);
    engine.set_input("kaifa");
    let full = candidates(&engine);
    engine.enter_aux();
    engine.push_aux_code('k');
    engine.push_aux_code('f');
    assert!(engine.backspace());
    assert_eq!(engine.aux_code(), "k");
    // 刪到空：回拼音態，`;` 從 preedit 消失，候選全回來
    assert!(engine.backspace());
    assert!(!engine.in_aux());
    assert_eq!(engine.query().unwrap().marked_text(), "kai'fa");
    assert_eq!(candidates(&engine), full);
    assert_eq!(engine.composition().text(), "kaifa");
}

/// 剛觸發（碼段空）就退格 = 退出輔碼態、拼音一個字元都不動。
#[test]
fn backspace_on_an_empty_code_leaves_the_pinyin_alone() {
    let mut engine = aux_engine();
    engine.set_input("kaifa");
    let full = candidates(&engine);
    engine.enter_aux();
    assert!(engine.backspace());
    assert!(!engine.in_aux());
    assert_eq!(engine.composition().text(), "kaifa");
    assert_eq!(candidates(&engine), full);
}

/// 拼音態退格刪拼音段末字元。
#[test]
fn backspace_in_pinyin_state_deletes_a_letter() {
    let mut engine = aux_engine();
    engine.set_input("kaifa");
    assert!(engine.backspace());
    assert_eq!(engine.composition().text(), "kaif");
}

/// Esc 在輔碼態清碼段（拼音與候選保持），在拼音態清拼音。
#[test]
fn escape_clears_the_code_then_the_pinyin() {
    let mut engine = aux_engine();
    engine.set_input("kaifa");
    let full = candidates(&engine);
    engine.enter_aux();
    engine.push_aux_code('k');
    engine.clear_aux();
    assert!(!engine.in_aux());
    assert_eq!(engine.composition().text(), "kaifa");
    assert_eq!(candidates(&engine), full);
    engine.clear();
    assert!(engine.composition().is_empty());
}

/// 輔碼態選詞上屏，碼段清空回初始態；學習按拼音段的全拼記，碼段與觸發鍵不在鍵裡。
#[test]
fn committing_from_aux_records_the_choice_under_the_pinyin() {
    let mut engine = aux_engine();
    engine.set_input("kaifa");
    engine.enter_aux();
    engine.push_aux_code('k');
    engine.push_aux_code('f');
    let candidate = engine.query().unwrap().candidates.items[0].clone();
    assert_eq!(candidate.text, "开发");
    engine.commit(&candidate);
    assert!(!engine.in_aux());
    assert!(engine.composition().is_empty());
    let last = engine.recent_commits.last().unwrap();
    assert!(last.same_input("kaifa"));
    assert_eq!(last.text, "开发");
}

/// 標點先上屏高亮候選（殼做），再走組句外標點語義；碼段隨之清空。
#[test]
fn punctuation_ends_the_aux_state() {
    let mut engine = aux_engine();
    engine.set_input("kaifa");
    engine.enter_aux();
    engine.push_aux_code('k');
    assert_eq!(engine.punctuate(';'), Some("；"));
    assert!(!engine.in_aux());
}

/// 回車原樣上屏拼音段，碼段與觸發鍵都不跟著上屏。
#[test]
fn take_raw_keeps_the_code_out_of_the_text() {
    let mut engine = aux_engine();
    engine.set_input("kaifa");
    engine.enter_aux();
    engine.push_aux_code('k');
    engine.push_aux_code('h');
    assert_eq!(engine.take_raw(), "kaifa");
    assert!(!engine.in_aux());
    assert!(engine.composition().is_empty());
}

/// 翻頁鍵與 `'` 不進碼段，殼按原語義處理。
#[test]
fn paging_keys_and_apostrophes_do_not_enter_the_code() {
    let mut engine = aux_engine();
    engine.set_input("kaifa");
    engine.enter_aux();
    for key in ['[', ']', ',', '.', '\'', '1', 'A'] {
        assert!(!engine.push_aux_code(key), "{key}");
    }
    assert_eq!(engine.aux_code(), "");
}

/// 輔碼態裡再敲觸發鍵是冪等的。
#[test]
fn triggering_again_is_idempotent() {
    let mut engine = aux_engine();
    engine.set_input("kaifa");
    engine.enter_aux();
    engine.push_aux_code('k');
    assert!(!engine.aux_trigger(';'));
    engine.enter_aux();
    assert_eq!(engine.aux_code(), "k");
}

/// 重排：完全匹配碼 > 碼長降序 > 原詞頻序。
#[test]
fn exact_code_matches_come_first_and_longer_codes_before_shorter_ones() {
    let table = Arc::new(
        AuxCodeTable::from_pairs([
            ("开".to_owned(), "kf".to_owned()),
            ("开发".to_owned(), "kfa".to_owned()),
            ("开放".to_owned(), "kfb".to_owned()),
        ])
        .unwrap(),
    );
    let mut engine = engine().with_aux_codes(vec![table]);
    engine.set_input("kai");
    engine.enter_aux();
    engine.push_aux_code('k');
    engine.push_aux_code('f');
    let query = engine.query().unwrap();
    let texts: Vec<&str> = query
        .candidates
        .items
        .iter()
        .map(|c| c.text.as_str())
        .collect();
    // 開 的碼正好是 kf（完全匹配），排在兩個字首命中的三段碼之前
    assert_eq!(texts.first(), Some(&"开"));
    assert!(texts.contains(&"开发") && texts.contains(&"开放"));
    assert_eq!(query.candidates.items[0].aux_code.as_deref(), Some("kf"));
    // 完全匹配的那條碼最前，其餘按碼長降序（這裡都是 3，保持原序）
    assert_eq!(query.candidates.items.len(), 3);
    engine.push_aux_code('a');
    assert_eq!(engine.aux_code(), "kfa");
    assert_eq!(candidates(&engine), ["开发"]);
}

/// 開關三態之一：開著但沒掛碼錶也不觸發（壞檔案全被裝配層跳過後同樣歸併到這裡），
/// `;` 完全保持原生行為；表來了才觸發。
#[test]
fn without_any_table_the_trigger_stays_off() {
    let mut bare = engine();
    bare.set_aux_enabled(true);
    bare.set_input("kaifa");
    assert!(!bare.aux_trigger(';'));
    bare.set_aux_codes(vec![codes()]);
    assert!(bare.aux_trigger(';'));
}

/// 開關三態之二：總開關預設關（`[aux_code] enabled`）——裝了碼錶也不觸發，
/// 開了才生效；關著時純拼音態也不逐候查首條碼（見 `pure_pinyin_skips_first_code_unless_shown`）。
#[test]
fn the_enabled_gate_defaults_off() {
    let mut engine = engine().with_aux_codes(vec![codes()]);
    engine.set_input("kaifa");
    assert!(!engine.aux_trigger(';'));
    engine.set_aux_enabled(true);
    assert!(engine.aux_trigger(';'));
}

/// 觸發鍵校驗：單字元標點、非字母數字、非 `'`（拼音隔音符）、非**當前**翻頁鍵——
/// `[ ]`、`,.` 只有真配成 `[general] page_keys` 才拒，換一套就放行。
#[test]
fn trigger_key_validation() {
    let paging = ('[', ']');
    assert!(is_valid_aux_code_key(';', paging));
    assert!(!is_valid_aux_code_key('\'', paging));
    assert!(!is_valid_aux_code_key('a', paging));
    assert!(!is_valid_aux_code_key('1', paging));
    assert!(!is_valid_aux_code_key('中', paging));
    // 翻頁鍵兩態：配成 [ ] 時 [ ] 拒；配成 ,. 時 [ ] 放行、, . 拒
    assert!(!is_valid_aux_code_key('[', paging));
    assert!(!is_valid_aux_code_key(']', paging));
    assert!(is_valid_aux_code_key('[', (',', '.')));
    assert!(is_valid_aux_code_key('.', ('[', ']')));
    assert!(!is_valid_aux_code_key('.', (',', '.')));
    let mut engine = aux_engine();
    assert_eq!(engine.aux_code_key(), ';');
    engine.set_aux_code_key('a', paging);
    assert_eq!(engine.aux_code_key(), ';');
    engine.set_aux_code_key('/', paging);
    assert_eq!(engine.aux_code_key(), '/');
    engine.set_input("kaifa");
    assert!(engine.aux_trigger('/'));
    assert!(!engine.aux_trigger(';'));
}

/// 輸入日誌按實際敲鍵記：拼音 + 觸發鍵 + 碼段一條鍵串，replay 逐鍵重喂不需要特殊邏輯；
/// 學習鍵仍是拼音段的全拼，碼段與觸發鍵不在裡面。
#[test]
fn the_input_log_records_the_trigger_and_the_code() {
    let entries = Arc::new(Mutex::new(Vec::new()));
    let mut engine = aux_engine().with_input_logger(Box::new(MemoryLogger(entries.clone())));
    engine.set_input("kaifa");
    engine.enter_aux();
    engine.push_aux_code('k');
    engine.push_aux_code('f');
    let candidate = engine.query().unwrap().candidates.items[0].clone();
    assert_eq!(candidate.text, "开发");
    engine.commit(&candidate);
    let entries = entries.lock().unwrap();
    let InputLogEntry::Commit(commit) = &entries[0] else {
        panic!("expected a commit");
    };
    assert_eq!(commit.keys, "kaifa;kf");
    assert_eq!(commit.scope, "kaifa");
    assert_eq!(commit.pinyin, "kai'fa");
}
