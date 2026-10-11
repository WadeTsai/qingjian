//! 輔碼在殼裡的閉環：觸發與逐鍵即篩、幀上的碼段、篩空與退格邊界、標點先上屏、
//! 碼錶熱載入、老 DLL 降級，以及自繪窗與 DLL 面各看各的幀。

use std::sync::Arc;

use qingjian_dictionary::{AuxCodeLookup, AuxCodeTable};
use qingjian_platform::Config;
use qingjian_platform::protocol::PreeditKind;
use qingjian_windows_server::dispatch::DataDirs;

use super::support::*;

/// 裝了輔碼碼錶的 Router：`kaifa` 之後敲觸發鍵進輔碼態、`k` 篩到 開發。
/// 碼錶由 Core 的 [`AuxCodeLookup`] 注入；輔碼總閘預設關，這裡顯式開啟。
fn router_with_aux(config: RouterConfig) -> Router {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let dict = root.join("assets/sample/dict.tsv");
    let glossary = root.join("assets/sample/glossary-en.tsv");
    let mut engine = assembly::assemble(&AssemblySpec {
        glossary: Some((Language::English, glossary)),
        english: Some(root.join("assets/sample/english.tsv")),
        ..AssemblySpec::new(dict)
    })
    .expect("assemble engine from sample data");
    let table: Arc<dyn AuxCodeLookup> = Arc::new(
        AuxCodeTable::from_pairs([
            ("开发".to_owned(), "kf".to_owned()),
            ("开发者".to_owned(), "kfz".to_owned()),
            ("开".to_owned(), "kh".to_owned()),
        ])
        .unwrap(),
    );
    engine.set_aux_codes(vec![table]);
    engine.set_aux_enabled(true);
    engine.set_aux_code_key(config.aux_code_key, config.page_keys);
    // 生產接線裡 Engine 側的同名開關與 RouterConfig 同源設定（main.rs），這裡對齊
    engine.set_shift_letter_compose(config.shift_letter_compose);
    let mut router = Router::new(engine, config);
    // 協議 6 起開會話回一條按鍵行為設定（SessionOpened），support 的 helper 負責吃掉它
    open_session(&mut router, SESSION, None);
    router
}

/// 預設配置 + 碼錶。
fn aux_router() -> Router {
    router_with_aux(RouterConfig::default())
}

/// 輔碼態：觸發鍵進狀態、碼段按 AuxCode 段下發、逐鍵即篩、篩空留 preedit、退格逐級放寬。
#[test]
fn aux_trigger_filters_candidates_and_marks_the_code_segment() {
    let mut router = aux_router();
    let (_, _, frame) = type_letters(&mut router, "kaifa");
    let before = candidate_texts(&frame).len();
    assert!(before > 1);

    let (outcome, committed, frame) = press(&mut router, letter(';'));
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert!(committed.is_none());
    assert_eq!(preedit(&frame), "kai'fa;");
    assert_eq!(candidate_texts(&frame).len(), before);
    assert!(!frame.preedit.iter().any(|s| s.kind == PreeditKind::AuxCode));

    let (_, _, frame) = press(&mut router, letter('k'));
    assert_eq!(preedit(&frame), "kai'fa;k");
    let segment = frame
        .preedit
        .iter()
        .find(|s| s.kind == PreeditKind::AuxCode)
        .expect("code segment rides the frame");
    assert_eq!(segment.text, "k");
    assert!(candidate_texts(&frame).contains(&"开发"));
    assert!(candidate_texts(&frame).contains(&"开发者"));
    assert!(!candidate_texts(&frame).contains(&"开放"));

    let (_, _, frame) = press(&mut router, letter('f'));
    assert_eq!(candidate_texts(&frame)[0], "开发");
    assert_eq!(frame.candidates.items[0].aux_code.as_deref(), Some("kf"));

    let (_, _, frame) = press(&mut router, letter('q'));
    assert!(frame.candidates.items.is_empty());
    assert!(!frame.is_empty());
    assert_eq!(preedit(&frame), "kai'fa;kfq");

    press(&mut router, function_key(0x08));
    press(&mut router, function_key(0x08));
    let (_, _, frame) = press(&mut router, function_key(0x08));
    assert_eq!(preedit(&frame), "kai'fa;");
    assert_eq!(candidate_texts(&frame).len(), before);
    let (_, _, frame) = press(&mut router, function_key(0x08));
    assert_eq!(preedit(&frame), "kai'fa");
    assert_eq!(candidate_texts(&frame).len(), before);
}

/// `[general] aux_code_key` 換成 `/`：`;` 不再是觸發鍵，`/` 才是。
#[test]
fn aux_trigger_key_follows_config() {
    let mut router = router_with_aux(RouterConfig {
        aux_code_key: '/',
        ..RouterConfig::default()
    });
    type_letters(&mut router, "kaifa");
    let (_, _, frame) = press(&mut router, letter(';'));
    assert!(frame.preedit.iter().all(|s| s.kind != PreeditKind::AuxCode));

    let mut router = router_with_aux(RouterConfig {
        aux_code_key: '/',
        ..RouterConfig::default()
    });
    type_letters(&mut router, "kaifa");
    let (_, _, frame) = press(&mut router, letter('/'));
    assert_eq!(preedit(&frame), "kai'fa/");
    assert!(!frame.preedit.iter().any(|s| s.kind == PreeditKind::AuxCode));
    let (_, _, frame) = press(&mut router, letter('k'));
    assert!(frame.preedit.iter().any(|s| s.kind == PreeditKind::AuxCode));
    assert!(candidate_texts(&frame).contains(&"开发"));
    let (_, _, frame) = press(&mut router, letter('h'));
    assert_eq!(candidate_texts(&frame), ["开"]);
}

/// 輔碼態裡敲標點：先把高亮候選上屏，再按組句外標點語義轉全形。
#[test]
fn punctuation_in_aux_commits_the_highlighted_candidate_first() {
    let mut router = aux_router();
    type_letters(&mut router, "kaifa");
    press(&mut router, letter(';'));
    press(&mut router, letter('k'));
    press(&mut router, letter('f'));
    let (_, committed, frame) = press(&mut router, punct(','));
    assert_eq!(committed.as_deref(), Some("开发，"));
    assert!(frame.candidates.items.is_empty());
}

/// 碼段篩空之後：空格 / 標點都不上屏原始拼音，吞掉停在輔碼態等退格。
#[test]
fn an_empty_filter_swallows_space_and_punctuation() {
    let mut router = aux_router();
    type_letters(&mut router, "kaifa");
    press(&mut router, letter(';'));
    press(&mut router, letter('k'));
    let (_, _, frame) = press(&mut router, letter('q'));
    assert!(frame.candidates.items.is_empty());

    let (outcome, committed, frame) = press(&mut router, letter(' '));
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert!(committed.is_none(), "筛空时空格不该上屏：{committed:?}");
    assert_eq!(preedit(&frame), "kai'fa;kq");

    let (_, committed, frame) = press(&mut router, punct(','));
    assert!(committed.is_none(), "筛空时标点不该上屏：{committed:?}");
    assert_eq!(preedit(&frame), "kai'fa;kq");

    let (_, _, frame) = press(&mut router, letter('1'));
    assert_eq!(preedit(&frame), "kai'fa;kq");

    press(&mut router, function_key(0x08));
    let (_, committed, frame) = press(&mut router, punct(','));
    let committed = committed.expect("有候选时标点该先上屏高亮候选");
    assert!(
        committed.starts_with("开发"),
        "上屏的是筛出来的候选：{committed}"
    );
    assert!(
        committed.ends_with('，'),
        "按组句外标点语义转全角：{committed}"
    );
    assert!(frame.candidates.items.is_empty());
}

/// Esc 在輔碼態只清碼段、拼音留著（不整段清掉）。
#[test]
fn escape_in_aux_only_clears_the_code() {
    let mut router = aux_router();
    type_letters(&mut router, "kaifa");
    press(&mut router, letter(';'));
    press(&mut router, letter('k'));
    press(&mut router, letter('f'));
    let (outcome, committed, frame) = press(&mut router, function_key(0x1B));
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert!(committed.is_none());
    assert_eq!(preedit(&frame), "kai'fa");
    assert!(frame.preedit.iter().all(|s| s.kind != PreeditKind::AuxCode));
    assert!(candidate_texts(&frame).contains(&"开发"));
}

/// `shift_letter = "compose"` 下輔碼態裡敲大寫：先清碼段回拼音態（與 Esc 同語義），字母照常收進緩衝區。
#[test]
fn uppercase_with_shift_letter_compose_exits_aux_before_pushing() {
    let mut router = router_with_aux(RouterConfig {
        shift_letter_compose: true,
        ..RouterConfig::default()
    });
    type_letters(&mut router, "kaifa");
    press(&mut router, letter(';'));
    press(&mut router, letter('k'));
    let (outcome, committed, frame) = press(&mut router, letter('G'));
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert!(committed.is_none());
    assert!(preedit(&frame).starts_with("kai'fa"));
    assert!(!preedit(&frame).contains(';'));
    assert!(frame.preedit.iter().all(|s| s.kind != PreeditKind::AuxCode));
}

/// `[general] aux_code_show` 隨幀下發（候選窗按它決定要不要拼碼註記）。
#[test]
fn aux_code_show_rides_the_frame() {
    let mut router = aux_router();
    let (_, _, frame) = type_letters(&mut router, "kaifa");
    assert!(!frame.aux_code_show);
    let mut router = router_with_aux(RouterConfig {
        aux_code_show: true,
        ..RouterConfig::default()
    });
    let (_, _, frame) = type_letters(&mut router, "kaifa");
    assert!(frame.aux_code_show);
}

/// 碼段用完刪到空之後：停在輔碼態、純拼音的候選一條不少地回來；再按一次退格才退出輔碼。
#[test]
fn deleting_the_whole_code_brings_every_candidate_back() {
    let mut router = aux_router();
    let (_, _, frame) = type_letters(&mut router, "kaifa");
    let before = candidate_texts(&frame).len();
    press(&mut router, letter(';'));
    press(&mut router, letter('k'));
    press(&mut router, letter('f'));
    press(&mut router, function_key(0x08));
    let (_, _, frame) = press(&mut router, function_key(0x08));
    assert_eq!(preedit(&frame), "kai'fa;");
    assert_eq!(candidate_texts(&frame).len(), before);
    let (_, _, frame) = press(&mut router, function_key(0x08));
    assert_eq!(preedit(&frame), "kai'fa");
    assert_eq!(candidate_texts(&frame).len(), before);
}

/// 輔碼態裡再敲一次觸發鍵是冪等的——留在輔碼態，既不上屏候選也不當標點。
#[test]
fn pressing_the_trigger_again_in_aux_mode_is_a_no_op() {
    let mut router = aux_router();
    type_letters(&mut router, "kaifa");
    press(&mut router, letter(';'));
    press(&mut router, letter('k'));
    let (outcome, committed, frame) = press(&mut router, letter(';'));
    assert_eq!(outcome, KeyOutcome::Consumed);
    assert!(committed.is_none());
    assert_eq!(preedit(&frame), "kai'fa;k");
    assert!(frame.candidates.items.iter().all(|c| c.text != "开发，"));
    let (_, _, frame) = press(&mut router, letter('f'));
    assert_eq!(preedit(&frame), "kai'fa;kf");
}

/// 老 DLL（協議 4）認不出 `AuxCode` 段：發給它那份把碼段降級成普通拼音段——
/// 輔碼篩選照常，只是不做淡色 + 下劃線的區分。不然老 DLL 會整條訊息解析失敗，
/// 把碼字母原樣放行給應用並斷開重連。
#[test]
fn an_old_dll_gets_the_code_segment_downgraded() {
    let mut router = aux_router();
    assert_eq!(
        router.handle(ClientMessage::OpenSession {
            session: SESSION,
            app: None,
            protocol: 4,
        }),
        None
    );
    type_letters(&mut router, "kaifa");
    press(&mut router, letter(';'));
    let (_, _, frame) = press(&mut router, letter('k'));
    assert_eq!(preedit(&frame), "kai'fa;k");
    assert!(
        frame.preedit.iter().all(|s| s.kind != PreeditKind::AuxCode),
        "老 DLL 不该收到 AuxCode 段：{:?}",
        frame.preedit
    );
    assert!(candidate_texts(&frame).contains(&"开发"));
    assert!(!candidate_texts(&frame).contains(&"开放"));
}

/// 降級只作用於發給 DLL 的那份：Server 自繪候選窗照常收到碼段（淡色 + 下劃線靠它）。
#[test]
fn the_self_drawn_window_keeps_the_code_segment_for_an_old_dll() {
    let mut router = aux_router();
    let sink = RecordingCandidates::default();
    router.set_candidate_sink(Box::new(sink.clone()));
    assert_eq!(
        router.handle(ClientMessage::OpenSession {
            session: SESSION,
            app: None,
            protocol: 4,
        }),
        None
    );
    type_letters(&mut router, "kaifa");
    let _ = router.handle(ClientMessage::PositionCandidates {
        session: SESSION,
        rect: rect(),
    });
    press(&mut router, letter(';'));
    let (_, _, frame) = press(&mut router, letter('k'));
    assert!(frame.preedit.iter().all(|s| s.kind != PreeditKind::AuxCode));
    let shown = sink.0.lock().unwrap();
    let last = shown.last().expect("自绘窗收到过帧");
    assert!(
        last.preedit.iter().any(|s| s.kind == PreeditKind::AuxCode),
        "自绘窗该看到码段：{:?}",
        last.preedit
    );
}

/// 設定頁剛匯入一張碼錶：`codes/` 目錄裡出現新檔案就重灌，不必等配置改動、也不必重啟。
#[test]
fn a_new_code_table_in_the_user_dir_hot_reloads() {
    let dir = std::env::temp_dir().join(format!("qingjian-codes-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let codes = dir.join("codes");
    std::fs::create_dir_all(&codes).unwrap();
    let mut router = router();
    router.engine_mut().set_aux_enabled(true);
    router.watch_config(
        &Config::default(),
        dir.join("config.toml"),
        dir.clone(),
        DataDirs {
            user_codes: Some(codes.clone()),
            ..DataDirs::default()
        },
    );

    // 一張表都沒有：總閘開著觸發鍵也不生效（`;` 保持原生行為）
    type_letters(&mut router, "kaifa");
    let (_, _, frame) = press(&mut router, letter(';'));
    assert!(frame.preedit.iter().all(|s| s.kind != PreeditKind::AuxCode));
    press(&mut router, function_key(0x1B));

    let table = AuxCodeTable::from_pairs([("开发".to_owned(), "kf".to_owned())]).unwrap();
    table
        .write_qj(&codes.join("mine.qj"), &Default::default())
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1200));
    router.poll_config_reload();

    // 碼錶進來：觸發鍵生效、逐鍵即篩到 開發
    type_letters(&mut router, "kaifa");
    let (_, _, frame) = press(&mut router, letter(';'));
    assert_eq!(preedit(&frame), "kai'fa;");
    let (_, _, frame) = press(&mut router, letter('k'));
    assert_eq!(candidate_texts(&frame), ["开发"]);
    let _ = std::fs::remove_dir_all(&dir);
}

/// 同一張碼錶被就地改寫（手工拷半個檔案後補全同理）：快照按逐檔案 mtime + 長度比，重灌生效。
#[test]
fn a_rewritten_code_table_hot_reloads() {
    let dir = std::env::temp_dir().join(format!("qingjian-codes-rw-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let codes = dir.join("codes");
    std::fs::create_dir_all(&codes).unwrap();
    let first = AuxCodeTable::from_pairs([("开发".to_owned(), "kf".to_owned())]).unwrap();
    first
        .write_qj(&codes.join("mine.qj"), &Default::default())
        .unwrap();

    let mut router = router();
    router.engine_mut().set_aux_enabled(true);
    router.watch_config(
        &Config::default(),
        dir.join("config.toml"),
        dir.clone(),
        DataDirs {
            user_codes: Some(codes.clone()),
            ..DataDirs::default()
        },
    );
    assert!(router.engine_mut().aux_codes().is_empty());

    let rewritten = AuxCodeTable::from_pairs([("开发".to_owned(), "kh".to_owned())]).unwrap();
    rewritten
        .write_qj(&codes.join("mine.qj"), &Default::default())
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1200));
    router.poll_config_reload();
    assert_eq!(router.engine_mut().aux_codes().len(), 1);

    type_letters(&mut router, "kaifa");
    press(&mut router, letter(';'));
    let (_, _, frame) = press(&mut router, letter('k'));
    assert!(candidate_texts(&frame).contains(&"开发"));
    let (_, _, frame) = press(&mut router, letter('h'));
    assert_eq!(candidate_texts(&frame), ["开发"]);
    let _ = std::fs::remove_dir_all(&dir);
}
