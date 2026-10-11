//! 形碼碼錶的裝載：`[general] scheme` 選了形碼時，把碼錶找出來掛到引擎上。
//!
//! 與本地整句模型同一套找法（見 [`super::find_model`]）：使用者目錄優先，隨包資料兜底。
//! 選形碼但碼錶不在時不靜默按拼音跑——配置說五筆、引擎還在拼音是很糟的錯位，寧可在日誌裡喊出來。

use std::path::{Path, PathBuf};

use qingjian_core::Engine;
use qingjian_dictionary::CodeTable;
use qingjian_platform::Scheme;

use super::Router;

/// 使用者目錄下的碼錶（使用者自己換的那份）。
const USER_TABLE: &str = "wubi/wubi86.tsv";

/// 隨包資料裡的碼錶。走 `assets/` 而不是 `data/`：這張表隨 git 跟蹤（`data/` 是給生成物的，
/// 裝機時從資料包解出來），與 emoji / levels 一樣在倉庫與安裝目錄裡是同一個相對路徑，
/// 所以 `cargo run` 的開發佈局也找得到。
const BUNDLED_TABLE: &str = "assets/wubi/wubi86.tsv";

/// 找碼錶：使用者目錄優先，否則隨包資料；都沒有為 `None`。
pub fn find_code_table(user_dir: Option<&Path>, bundled_root: &Path) -> Option<PathBuf> {
    user_dir
        .map(|dir| dir.join(USER_TABLE))
        .filter(|path| path.is_file())
        .or_else(|| {
            let bundled = bundled_root.join(BUNDLED_TABLE);
            bundled.is_file().then_some(bundled)
        })
}

/// 按配置的兩條軸裝配引擎：拼音側（全拼 / 雙拼 / 注音 / 關）與形碼側（五筆）。
/// 兩邊都開就是混輸——編碼打全的形碼候選在前，見 `Engine::query_mixed`。
///
/// 沒開形碼時把碼錶卸掉；開了但碼錶不在就警告並退回只用拼音（配置說五筆、引擎一個字都打不出
/// 更糟）。`table` 是啟動時用 [`find_code_table`] 找好的路徑（與本地模型一樣，熱載入時不重新找）。
fn apply_scheme(engine: &mut Engine, pinyin: Scheme, wubi: bool, table: Option<&Path>) {
    engine.set_shuangpin(pinyin.shuangpin());
    engine.set_zhuyin_mode(pinyin == Scheme::Zhuyin);
    // 拼音側關掉且形碼開著才是「只用形碼」；兩邊都關著時留拼音兜底（否則一個候選都沒有）
    engine.set_phonetic(pinyin.is_on() || !wubi);
    if !wubi {
        engine.set_code_table(None);
        return;
    }
    let Some(path) = table else {
        tracing::warn!(
            table = BUNDLED_TABLE,
            "选了形码方案但找不到码表，仍按拼音输入；随包数据里应当带一份",
        );
        engine.set_code_table(None);
        return;
    };
    match CodeTable::from_path(path) {
        Ok(table) => {
            tracing::info!(table = %path.display(), entries = table.len(), "形码码表已载入");
            engine.set_code_table(Some(table));
        }
        Err(error) => {
            tracing::error!(%error, table = %path.display(), "形码码表读不了，仍按拼音输入");
            engine.set_code_table(None);
        }
    }
}

impl Router {
    /// 啟動時把找好的碼錶路徑交給路由器，並按當前配置裝配一次。
    pub fn configure_code_table(&mut self, table: Option<PathBuf>) {
        self.code_table = table;
        let scheme = self.scheme();
        apply_scheme(
            &mut self.engine,
            scheme,
            self.config.wubi,
            self.code_table.as_deref(),
        );
    }

    /// 熱載入與焦點換到另一個 profile 的會話：方案變了就按同一個路徑重新裝配（不重新找檔案，與本地模型一致）。
    /// 拼音側傳聚焦會話算出來的方案（`Router::scheme`），不直接傳配置的：注音 profile 蓋過配置。
    pub(super) fn reload_code_table(&mut self, pinyin: Scheme, wubi: bool) {
        apply_scheme(&mut self.engine, pinyin, wubi, self.code_table.as_deref());
    }
}
