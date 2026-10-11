//! 主題庫：內建主題加使用者主題目錄裡的主題，設定介面列出的就是它。
//!
//! 使用者主題放在 `<主題目錄>/<id>/theme.json`（主題目錄由殼給，在配置目錄下的 `themes/`）。目錄名必須等於 `meta.id`；
//! 不能與內建主題重名（想改內建主題就 `extends` 它、換個 id），外觀詞 `system` / `light` / `dark` 也不能當 id。
//! 讀不進來的主題跳過並記警告，不影響其他主題。
//!
//! 熱載入：殼在已有的每秒檢查裡調 [`ThemeLibrary::refresh`]，目錄裡 `theme.json` 的增刪改（按修改時間與大小）會重讀。

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use super::Theme;

/// 使用者主題的描述檔名。
const THEME_FILE: &str = "theme.json";

/// 外觀的寫法，不能當主題 id（舊配置裡 `theme` 寫的是它們）。
const RESERVED_IDS: [&str; 3] = ["system", "light", "dark"];

pub struct ThemeLibrary {
    /// 內建主題在前、使用者主題按 id 排在後，都是淺色那一份。
    themes: Vec<Theme>,

    /// 使用者主題目錄；`None` 只有內建主題。
    dir: Option<PathBuf>,

    /// 上次讀目錄時的戳，見 [`Self::stamp`]。
    stamp: u64,
}

impl ThemeLibrary {
    /// 讀內建主題與 `dir` 裡的使用者主題。目錄不存在就只有內建主題。
    pub fn load(dir: Option<&Path>) -> Self {
        let stamp = dir.map_or(0, Self::stamp);
        let mut themes = Theme::builtins().to_vec();
        if let Some(dir) = dir {
            themes.extend(read_user_themes(dir));
        }
        Self {
            themes,
            dir: dir.map(Path::to_path_buf),
            stamp,
        }
    }

    /// 全部主題（淺色），內建在前。
    pub fn themes(&self) -> &[Theme] {
        &self.themes
    }

    /// 按 id 取主題並換成對應外觀；沒有這個 id 時退回預設主題並記警告。
    pub fn resolve(&self, id: &str, dark: bool) -> Theme {
        match self.themes.iter().find(|theme| theme.id() == id) {
            Some(theme) => theme.with_dark(dark),
            None => {
                tracing::warn!(id, "没有这个主题，用預設主题");
                Theme::light().with_dark(dark)
            }
        }
    }

    /// 使用者主題目錄有變化就重讀，返回是否重讀了。
    pub fn refresh(&mut self) -> bool {
        let Some(dir) = &self.dir else {
            return false;
        };
        let stamp = Self::stamp(dir);
        if stamp == self.stamp {
            return false;
        }
        *self = Self::load(Some(&dir.clone()));
        true
    }

    /// 目錄戳：各主題子目錄名與 `theme.json` 的修改時間、大小合成一個數，任何一個變了它就變。
    /// 只 `stat` 不讀內容，每秒查一次的開銷可以忽略。
    pub fn stamp(dir: &Path) -> u64 {
        let mut hasher = DefaultHasher::new();
        for (name, path) in theme_dirs(dir) {
            name.hash(&mut hasher);
            if let Ok(meta) = std::fs::metadata(path.join(THEME_FILE)) {
                meta.len().hash(&mut hasher);
                meta.modified().ok().hash(&mut hasher);
            }
        }
        hasher.finish()
    }
}

/// 主題目錄下的子目錄（名字、路徑），按名字排序。
fn theme_dirs(dir: &Path) -> Vec<(String, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut dirs: Vec<(String, PathBuf)> = entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| Some((entry.file_name().into_string().ok()?, entry.path())))
        .collect();
    dirs.sort();
    dirs
}

/// 讀使用者主題，讀不進來或不合規矩的跳過並記警告。
fn read_user_themes(dir: &Path) -> Vec<Theme> {
    let mut themes = Vec::new();
    for (name, path) in theme_dirs(dir) {
        let file = path.join(THEME_FILE);
        if !file.is_file() {
            continue;
        }
        let theme = match Theme::from_dir(&path, false) {
            Ok(theme) => theme,
            Err(error) => {
                tracing::warn!(path = %file.display(), %error, "用户主题读不进来，跳过");
                continue;
            }
        };
        let problem = if theme.id() != name {
            Some("meta.id 与目录名不一致")
        } else if RESERVED_IDS.contains(&theme.id()) {
            Some("外观词不能当主题 id")
        } else if Theme::builtin(theme.id(), false).is_some() {
            Some("与内置主题重名")
        } else {
            None
        };
        match problem {
            Some(problem) => {
                tracing::warn!(path = %file.display(), id = theme.id(), "用户主题{problem}，跳过");
            }
            None => themes.push(theme),
        }
    }
    themes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_theme(dir: &Path, name: &str, id: &str) {
        let path = dir.join(name);
        std::fs::create_dir_all(&path).unwrap();
        let json = format!(
            r##"{{ "extends": "qingjian", "meta": {{ "id": "{id}", "name": "{id} 主题" }},
                "variables": {{ "accent": "#ff0000" }} }}"##
        );
        std::fs::write(path.join(THEME_FILE), json).unwrap();
    }

    #[test]
    fn loads_user_themes_after_builtins_and_skips_bad_ones() {
        let dir = std::env::temp_dir().join("qingjian-theme-library-test");
        let _ = std::fs::remove_dir_all(&dir);
        write_theme(&dir, "peach", "peach");
        write_theme(&dir, "mismatch", "other");
        write_theme(&dir, "wechat", "wechat");
        write_theme(&dir, "dark", "dark");
        std::fs::create_dir_all(dir.join("broken")).unwrap();
        std::fs::write(dir.join("broken").join(THEME_FILE), "{").unwrap();

        let mut library = ThemeLibrary::load(Some(&dir));
        let ids: Vec<&str> = library.themes().iter().map(Theme::id).collect();
        let builtin = Theme::builtins().len();
        assert_eq!(ids.len(), builtin + 1);
        assert_eq!(ids[builtin], "peach");
        assert_eq!(library.resolve("peach", true).id(), "peach");
        assert_eq!(library.resolve("nope", false).id(), "qingjian");
        assert!(!library.refresh());

        write_theme(&dir, "maple", "maple");
        assert!(library.refresh());
        assert!(library.themes().iter().any(|theme| theme.id() == "maple"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
