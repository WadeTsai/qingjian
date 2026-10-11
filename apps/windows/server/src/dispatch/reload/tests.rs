//! 詞庫檔案變化在配置不變時仍應進入正在執行的引擎。

use std::path::Path;
use std::time::{Duration, Instant, SystemTime};

use qingjian_core::Engine;
use qingjian_dictionary::{Dictionary, import};
use qingjian_platform::Config;

use super::CONFIG_POLL_INTERVAL;
use crate::dispatch::{DataDirs, Router, RouterConfig};

fn poll(router: &mut Router) {
    router.reload.as_mut().unwrap().last_check = Instant::now() - CONFIG_POLL_INTERVAL;
    router.poll_config_reload();
}

fn modified_at(path: &Path, seconds: u64) {
    std::fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(seconds))
        .unwrap();
}

#[test]
fn import_replace_and_remove_without_config_changes() {
    let dir = std::env::temp_dir().join(format!("qingjian-reload-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let config_path = dir.join("config.toml");
    std::fs::write(&config_path, "").unwrap();
    let config = Config::load(&config_path).unwrap();
    let original_config = std::fs::read(&config_path).unwrap();
    let mut router = Router::new(Engine::new(Dictionary::default()), RouterConfig::default());
    router.watch_config(
        &config,
        config_path.clone(),
        dir.clone(),
        DataDirs {
            user_root: Some(dir.clone()),
            user_dicts: Some(dir.join("dicts")),
            ..DataDirs::default()
        },
    );

    let source = dir.join("law.dict.yaml");
    std::fs::write(&source, "---\nname: law\n...\n合同法\the tong fa\t120\n").unwrap();
    let imported = import::import(&source, &dir.join("dicts")).unwrap();
    modified_at(&imported.path, 100);
    poll(&mut router);
    let dictionaries = router.engine.extra_dictionaries();
    assert_eq!(dictionaries.len(), 1);
    assert_eq!(
        dictionaries[0].lookup(&["he", "tong", "fa"], false)[0].text,
        "合同法"
    );

    // 只改詞頻，輸出長度保持不變，必須靠 mtime 發現更新。
    let original_len = std::fs::metadata(&imported.path).unwrap().len();
    std::fs::write(&source, "---\nname: law\n...\n合同法\the tong fa\t121\n").unwrap();
    import::import(&source, &dir.join("dicts")).unwrap();
    modified_at(&imported.path, 200);
    assert_eq!(
        std::fs::metadata(&imported.path).unwrap().len(),
        original_len
    );
    poll(&mut router);
    assert_eq!(
        router.engine.extra_dictionaries()[0].lookup(&["he", "tong", "fa"], false)[0].frequency,
        121
    );

    // 保持舊詞庫對映開啟，模擬執行中的 Server 收到同名替換。
    std::fs::write(
        &source,
        "---\nname: law\n...\n民法典\tmin fa dian\t100\n法律\tfa lv\t30\n",
    )
    .unwrap();
    import::import(&source, &dir.join("dicts")).unwrap();
    poll(&mut router);
    let dictionary = &router.engine.extra_dictionaries()[0];
    assert!(dictionary.lookup(&["he", "tong", "fa"], false).is_empty());
    assert_eq!(
        dictionary.lookup(&["min", "fa", "dian"], false)[0].text,
        "民法典"
    );

    // YAML 頭中只有引用、沒有本檔案詞條時，失敗不能覆蓋已有詞庫。
    std::fs::write(&source, "---\nname: law\nimport_tables:\n  - other\n...\n").unwrap();
    assert!(import::import(&source, &dir.join("dicts")).is_err());
    assert_eq!(Dictionary::from_path(&imported.path).unwrap().len(), 2);

    let removed = dir.join("dicts/removed");
    std::fs::create_dir_all(&removed).unwrap();
    std::fs::rename(&imported.path, removed.join("law.qj")).unwrap();
    poll(&mut router);
    assert!(router.engine.extra_dictionaries().is_empty());
    assert_eq!(std::fs::read(&config_path).unwrap(), original_config);
    drop(router);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn dictionary_changes_do_not_retry_broken_config() {
    let dir = std::env::temp_dir().join(format!("qingjian-reload-broken-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let config_path = dir.join("config.toml");
    std::fs::write(&config_path, "[general]\npage_size = 5\n").unwrap();
    modified_at(&config_path, 100);
    let config = Config::load(&config_path).unwrap();
    let mut router = Router::new(
        Engine::new(Dictionary::default()),
        RouterConfig::from(&config),
    );
    router.watch_config(
        &config,
        config_path.clone(),
        dir.clone(),
        DataDirs {
            user_root: Some(dir.clone()),
            user_dicts: Some(dir.join("dicts")),
            ..DataDirs::default()
        },
    );

    std::fs::write(&config_path, "[broken").unwrap();
    modified_at(&config_path, 200);
    poll(&mut router);
    let source = dir.join("law.dict.yaml");
    std::fs::write(&source, "---\nname: law\n...\n合同法\the tong fa\t120\n").unwrap();
    let imported = import::import(&source, &dir.join("dicts")).unwrap();
    poll(&mut router);
    assert_eq!(router.config.page_size, 5);
    assert_eq!(router.engine.extra_dictionaries().len(), 1);

    // 保持壞配置的 mtime，以可觀察的配置值確認後續輪詢不會重新讀它。
    std::fs::write(&config_path, "[general]\npage_size = 9\n").unwrap();
    modified_at(&config_path, 200);
    poll(&mut router);
    assert_eq!(router.config.page_size, 5);
    let removed = dir.join("dicts/removed");
    std::fs::create_dir_all(&removed).unwrap();
    std::fs::rename(&imported.path, removed.join("law.qj")).unwrap();
    poll(&mut router);
    assert_eq!(router.config.page_size, 5);
    assert!(router.engine.extra_dictionaries().is_empty());

    modified_at(&config_path, 300);
    poll(&mut router);
    assert_eq!(router.config.page_size, 9);
    drop(router);
    std::fs::remove_dir_all(&dir).unwrap();
}
