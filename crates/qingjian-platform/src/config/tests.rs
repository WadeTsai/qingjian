//! 配置的讀寫測試：模板、分節解析、舊寫法相容、原地改鍵。

use super::*;

#[test]
fn template_parses_to_defaults() {
    let config: Config = toml::from_str(TEMPLATE).unwrap();
    assert_eq!(config, Config::default());
}

#[test]
fn partial_file_keeps_other_defaults() {
    let config: Config = toml::from_str("[predict]\nenabled = true\nlookback = 10\n").unwrap();
    assert!(config.predict.enabled);
    assert_eq!(config.predict.lookback, 10);
    assert_eq!(config.predict.model, "deepseek-v4-flash");
    assert_eq!(config.predict.reasoning_effort, "none");
    assert_eq!(config.predict.api_key_env, "QINGJIAN_API_KEY");
}

#[test]
fn fuzzy_section_parses() {
    let config: Config = toml::from_str("[fuzzy]\nz_zh = true\nan_ang = true\n").unwrap();
    assert!(config.fuzzy.z_zh && config.fuzzy.an_ang && !config.fuzzy.n_l);
    assert!(config.fuzzy.any());
}

#[test]
fn general_and_shortcut_sections_parse() {
    let config: Config = toml::from_str(
        "[general]\npage_size = 5\npage_keys = \"[]\"\ntheme = \"dark\"\nlayout = \"horizontal\"\npreedit = \"window\"\n[shortcut]\nexpression = \"i\"\n",
    )
    .unwrap();
    assert_eq!(config.general.page_size(), 5);
    assert_eq!(config.general.page_keys(), ('[', ']'));
    // 舊寫法：外觀寫在 theme 裡
    assert_eq!(config.general.appearance(), Appearance::Dark);
    assert_eq!(config.general.theme_id(), DEFAULT_THEME);
    assert_eq!(config.general.layout, LayoutMode::Horizontal);
    assert_eq!(config.general.preedit, PreeditMode::Window);
    assert_eq!(config.general.learning_language, "en");
    assert!(config.general.english_candidates);
    assert!(config.general.emoji);
    assert!(config.general.english_in_chinese);
    assert!(!config.general.traditional);
    assert_eq!(config.general.shuangpin(), None);
    assert_eq!(config.general.log_level, LogLevel::Info);
    assert_eq!(config.shortcut.mode.expression, 'i');
    assert_eq!(config.shortcut.mode.question, 'u');
    // 譯詞修飾鍵預設分平臺（Windows 是 Ctrl 系，其餘 Option 系，見 shortcut.rs），斷言跟著 Default 走
    assert_eq!(
        config.shortcut.translation,
        Config::default().shortcut.translation
    );
    assert_eq!(config.shortcut.switch_mode, SwitchKeys::default());
    assert!(config.general.english_mode);
}

#[test]
fn appearance_reads_legacy_theme_key() {
    let parse = |text: &str| toml::from_str::<Config>(text).unwrap().general;
    // 舊寫法：外觀寫在 theme 裡
    let legacy = parse("[general]\ntheme = \"light\"\n");
    assert_eq!(legacy.appearance(), Appearance::Light);
    assert_eq!(legacy.theme_id(), DEFAULT_THEME);
    // 新舊都在（設定頁寫了 appearance，舊的 theme 還留著）：appearance 為準
    let both = parse("[general]\ntheme = \"dark\"\nappearance = \"system\"\n");
    assert_eq!(both.appearance(), Appearance::System);
    assert_eq!(both.theme_id(), DEFAULT_THEME);
    // 新寫法
    let new = parse("[general]\nappearance = \"dark\"\ntheme = \"sakura\"\n");
    assert_eq!(new.appearance(), Appearance::Dark);
    assert_eq!(new.theme_id(), "sakura");
    // 都沒寫
    let empty = parse("[general]\n");
    assert_eq!(empty.appearance(), Appearance::System);
    assert_eq!(empty.theme_id(), DEFAULT_THEME);
}

#[test]
fn writing_appearance_migrates_legacy_theme_line() {
    assert!(TEMPLATE.contains(&format!("{APPEARANCE_COMMENT}appearance = ")));
    assert!(TEMPLATE.contains(&format!("{THEME_COMMENT}theme = ")));
    let path = std::env::temp_dir().join("qingjian-config-migrate-theme-test.toml");
    std::fs::write(
        &path,
        "[general]\n# 候選視窗外觀：system 跟隨系統 / light 淺色 / dark 深色\ntheme = \"dark\"\nlayout = \"vertical\"\n",
    )
    .unwrap();
    Config::set_value(&path, "general", "appearance", "light").unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains(&format!("{THEME_COMMENT}theme = \"qingjian\"\nlayout")));
    assert!(text.contains(&format!("{APPEARANCE_COMMENT}appearance = \"light\"")));
    assert_eq!(text.matches("候選視窗外觀").count(), 1);
    let general = Config::load(&path).unwrap().general;
    assert_eq!(general.appearance(), Appearance::Light);
    assert_eq!(general.theme_id(), DEFAULT_THEME);
    // 已遷移過（或新寫法）的不再動
    Config::set_value(&path, "general", "appearance", "dark").unwrap();
    let again = std::fs::read_to_string(&path).unwrap();
    assert_eq!(
        again,
        text.replace("appearance = \"light\"", "appearance = \"dark\"")
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn writing_theme_first_keeps_legacy_appearance() {
    let path = std::env::temp_dir().join("qingjian-config-migrate-theme-first-test.toml");
    std::fs::write(
        &path,
        "[general]\n# 候選視窗外觀：system 跟隨系統 / light 淺色 / dark 深色\ntheme = \"dark\"\n",
    )
    .unwrap();
    // 沒切過外觀、直接選了主題：舊的深色不能丟，註釋也要換掉
    Config::set_value(&path, "general", "theme", "wechat").unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains(&format!("{THEME_COMMENT}theme = \"wechat\"")));
    assert!(text.contains(&format!("{APPEARANCE_COMMENT}appearance = \"dark\"")));
    assert_eq!(text.matches("候選視窗外觀").count(), 1);
    let general = Config::load(&path).unwrap().general;
    assert_eq!(general.appearance(), Appearance::Dark);
    assert_eq!(general.theme_id(), "wechat");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_value_writes_strings_and_integers() {
    let path = std::env::temp_dir().join("qingjian-config-set-value-test.toml");
    let _ = std::fs::remove_file(&path);
    Config::set_value(&path, "general", "page_size", 5i64).unwrap();
    Config::set_value(&path, "general", "appearance", "dark").unwrap();
    Config::set_value(&path, "shortcut", "question", "i").unwrap();
    let config = Config::load(&path).unwrap();
    assert_eq!(config.general.page_size, 5);
    assert_eq!(config.general.appearance(), Appearance::Dark);
    assert_eq!(config.shortcut.mode.question, 'i');
    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_bool_keeps_comments_and_flips_only_that_key() {
    let path = std::env::temp_dir().join("qingjian-config-set-bool-test.toml");
    std::fs::write(
        &path,
        "# 頭註釋\n[fuzzy]\n# 說明\nz_zh = false\nn_l = true\n",
    )
    .unwrap();
    Config::set_bool(&path, "fuzzy", "z_zh", true).unwrap();
    Config::set_bool(&path, "predict", "enabled", true).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(
        text.starts_with("# 頭註釋\n[fuzzy]\n# 說明\nz_zh = true\nn_l = true\n"),
        "{text}"
    );
    let config = Config::load(&path).unwrap();
    assert!(config.fuzzy.z_zh && config.fuzzy.n_l && config.predict.enabled);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn writes_create_the_data_directory_for_a_fresh_account() {
    let dir = std::env::temp_dir().join("qingjian-config-fresh-account-test");
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("Qingjian").join("config.toml");
    assert!(Config::write_template_if_missing(&path).unwrap());
    assert!(!Config::write_template_if_missing(&path).unwrap());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), TEMPLATE);
    // 沒有模板直接儲存也行
    std::fs::remove_dir_all(&dir).unwrap();
    Config::set_bool(&path, "predict", "enabled", true).unwrap();
    assert!(Config::load(&path).unwrap().predict.enabled);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn set_bool_starts_from_template_when_missing() {
    let path = std::env::temp_dir().join("qingjian-config-set-bool-missing-test.toml");
    let _ = std::fs::remove_file(&path);
    Config::set_bool(&path, "predict", "enabled", true).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("# 青簡輸入法配置"));
    assert!(Config::load(&path).unwrap().predict.enabled);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn set_bool_refuses_broken_file() {
    let path = std::env::temp_dir().join("qingjian-config-set-bool-broken-test.toml");
    std::fs::write(&path, "[fuzzy\nz_zh = false\n").unwrap();
    assert!(matches!(
        Config::set_bool(&path, "fuzzy", "z_zh", true),
        Err(ConfigError::Edit { .. })
    ));
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "[fuzzy\nz_zh = false\n"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn missing_file_is_default() {
    let path = std::env::temp_dir().join("qingjian-config-missing-test.toml");
    let _ = std::fs::remove_file(&path);
    assert_eq!(Config::load(&path).unwrap(), Config::default());
}

#[test]
fn font_sizes_and_animations_parse_and_default() {
    let config: Config = toml::from_str(
        "[general]\ncandidate_font_size = 20\nannotation_font_size = 13.5\nanimations = false\n",
    )
    .unwrap();
    assert_eq!(config.general.candidate_font_size.get(), Some(20.0));
    assert_eq!(config.general.annotation_font_size.get(), Some(13.5));
    assert!(!config.general.animations);
    let default = Config::default();
    assert_eq!(default.general.candidate_font_size.get(), None);
    assert!(default.general.animations);
    assert_eq!(FontSize(-3.0).get(), None);
}

#[test]
fn font_size_edits_ignore_echoed_theme_size() {
    let unset = FontSize::default();
    // 框裡顯示主題的 17，原樣交回不寫
    assert_eq!(unset.edited(Some(17.0), Some(17.0)), None);
    assert_eq!(unset.edited(Some(20.0), Some(17.0)), Some(FontSize(20.0)));
    // 設過 20：清空或填 0 回到主題的
    assert_eq!(FontSize(20.0).edited(None, Some(17.0)), Some(FontSize(0.0)));
    assert_eq!(
        FontSize(20.0).edited(Some(0.0), Some(17.0)),
        Some(FontSize(0.0))
    );
    // 設過 20 再填主題的 17：照寫（使用者明確要 17）
    assert_eq!(
        FontSize(20.0).edited(Some(17.0), Some(17.0)),
        Some(FontSize(17.0))
    );
    assert_eq!(unset.edited(None, Some(17.0)), None);
}
