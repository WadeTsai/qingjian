//! 字型庫：不掃系統字型目錄（fontdb 全掃幾百毫秒、幾十 MB），按平臺清單隻載入介面字型、中文、日文、emoji 幾個檔案。
//!
//! 檔案走 fontdb 的 mmap 載入，只解析名字表與 cmap，Apple Color Emoji 那種 190 MB 的檔案也只在用到字形時才讀頁。
//! 中日同形字按 locale 回退（cosmic-text 的平臺回退表：zh-CN → PingFang SC，ja → Hiragino Sans）。

#[cfg(target_os = "windows")]
pub mod directwrite;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
mod trak;
mod ui_font;
#[cfg(target_os = "windows")]
mod windows;

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use cosmic_text::FontSystem;
use cosmic_text::fontdb::{Database, Family, Source};

use crate::error::RenderError;

pub(crate) use trak::Trak;
pub use ui_font::UiFont;

#[cfg(target_os = "windows")]
use self::windows as platform;
#[cfg(target_os = "linux")]
use linux as platform;
#[cfg(target_os = "macos")]
use macos as platform;

pub struct FontLibrary {
    /// 已載入的字型。
    db: Database,

    /// 介面字型的字族名（`SansSerif` 對映到它）。
    ui_family: String,

    /// 中日同形字回退用的 locale，如 `zh-CN`。
    locale: String,
}

impl FontLibrary {
    /// 按平臺清單載入系統字型。`locale` 決定 Han 字形選哪家（`zh-CN` / `zh-TW` / `ja`）。
    pub fn system(locale: &str) -> Result<Self, RenderError> {
        Self::build(locale, None)
    }

    /// 介面字型換成使用者指定的字族，系統字型仍載入在後面當回退；指定的字型一個檔案都沒載入到或名字對不上就退回系統字型。
    pub fn with_ui_font(locale: &str, ui_font: &UiFont) -> Result<Self, RenderError> {
        Self::build(locale, Some(ui_font))
    }

    fn build(locale: &str, ui_font: Option<&UiFont>) -> Result<Self, RenderError> {
        let mut db = Database::new();
        let custom_family = ui_font.and_then(|font| {
            let loaded = font.files.iter().filter(|path| load(&mut db, path)).count();
            let found = db.faces().any(|face| {
                face.families
                    .iter()
                    .any(|(name, _)| name.eq_ignore_ascii_case(&font.family))
            });
            if loaded > 0 && found {
                Some(font.family.clone())
            } else {
                tracing::warn!(
                    family = font.family,
                    files = font.files.len(),
                    loaded,
                    "指定的候选窗字体没找到，用系统字体"
                );
                None
            }
        });
        let ui =
            load_first(&mut db, &platform::ui_fonts()).ok_or_else(|| RenderError::NoUiFont {
                tried: platform::ui_fonts(),
            })?;
        let ui_family = custom_family.unwrap_or_else(|| {
            db.face(ui)
                .and_then(|face| face.families.first().map(|(name, _)| name.clone()))
                .unwrap_or_else(|| "sans-serif".to_owned())
        });
        db.set_sans_serif_family(ui_family.clone());
        // 中文指令碼字型必須真有 swash 能柵格化的輪廓（macOS 26+ 的蘋方是私有的 hvgl 可變輪廓，讀不出字形）；
        // emoji 走 sbix 點陣圖、沒有輪廓表，不做這項檢查
        for path in platform::script_fonts(locale) {
            load_outline_font(&mut db, &path);
        }
        for path in platform::ui_weight_fonts()
            .into_iter()
            .chain(platform::emoji_fonts())
        {
            if !load(&mut db, &path) {
                tracing::debug!(path = %path.display(), "字体文件不存在，跳过");
            }
        }
        tracing::debug!(faces = db.len(), ui_family, "字体库就绪");
        Ok(Self {
            db,
            ui_family,
            locale: locale.to_owned(),
        })
    }

    /// 已載入的字族名，按載入順序去重。
    pub fn families(&self) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        for face in self.db.faces() {
            for (name, _) in &face.families {
                if !names.contains(name) {
                    names.push(name.clone());
                }
            }
        }
        names
    }

    /// 已載入的字型檔案，按載入順序去重。快照測試拿它判斷字型環境變沒變。
    pub fn font_files(&self) -> Vec<PathBuf> {
        let mut files: Vec<PathBuf> = Vec::new();
        for face in self.db.faces() {
            let path = match &face.source {
                Source::File(path) | Source::SharedFile(path, _) => path,
                Source::Binary(_) => continue,
            };
            if !files.contains(path) {
                files.push(path.clone());
            }
        }
        files
    }

    pub fn ui_family(&self) -> &str {
        &self.ui_family
    }

    /// 交給 cosmic-text。
    pub(crate) fn into_font_system(self) -> FontSystem {
        let mut db = self.db;
        db.set_sans_serif_family(self.ui_family);
        FontSystem::new_with_locale_and_db(self.locale, db)
    }
}

/// 介面字型用的字族。
pub(crate) const UI_FAMILY: Family<'static> = Family::SansSerif;

/// 依次嘗試，載入成功的第一個檔案的第一張面。
fn load_first(db: &mut Database, paths: &[PathBuf]) -> Option<cosmic_text::fontdb::ID> {
    paths.iter().find_map(|path| {
        let before: Vec<_> = db.faces().map(|f| f.id).collect();
        load(db, path)
            .then(|| db.faces().map(|f| f.id).find(|id| !before.contains(id)))
            .flatten()
    })
}

/// 檔案存在且能解析就載入；返回是否載入了。
fn load(db: &mut Database, path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    match db.load_font_file(path) {
        Ok(()) => true,
        Err(error) => {
            tracing::warn!(path = %path.display(), %error, "字体文件解析失败");
            false
        }
    }
}

/// 存在、能解析、且至少有一張面帶 swash 能柵格化的輪廓（`glyf` / `CFF `）才載入。
fn load_outline_font(db: &mut Database, path: &Path) {
    let outline = File::open(path).and_then(|mut file| has_rasterizable_outline(&mut file));
    match outline {
        Ok(true) => {
            load(db, path);
        }
        Ok(false) => {
            tracing::warn!(path = %path.display(), "字体轮廓是 swash 读不了的格式（如 macOS 26+ 苹方的 hvgl），跳过");
        }
        Err(error) => {
            tracing::debug!(path = %path.display(), %error, "字体文件不存在或读不了，跳过");
        }
    }
}

/// 檔案裡任一面的輪廓表是 swash 能柵格化的 `glyf` / `CFF `。macOS 26+ 的蘋方（PingFangUI.ttc）把輪廓
/// 換成了私有的 `hvgl`（可變字型，無公開文件，googlefonts/fontations#1369 在跟進支援），swash 讀不出
/// 任何字形，中文候選會畫成空白；這種檔案不載入，讓回退落到清單裡還能柵格化的字型上。
fn has_rasterizable_outline(file: &mut (impl Read + Seek)) -> std::io::Result<bool> {
    let mut head = [0; 12];
    file.read_exact(&mut head)?;
    // TTC：tag(4) 版本(4) 面數(4) 後跟每張面 sfnt 頭的偏移；單 sfnt 檔案只有開頭一張面
    let face_offsets: Vec<u64> = if head.starts_with(b"ttcf") {
        let count = u32::from_be_bytes(head[8..].try_into().expect("head 有 12 字节")).min(4096);
        (0..count)
            .map(|i| {
                let mut offset = [0; 4];
                file.seek(SeekFrom::Start(u64::from(i) * 4 + 12))?;
                file.read_exact(&mut offset)?;
                Ok(u64::from(u32::from_be_bytes(offset)))
            })
            .collect::<std::io::Result<_>>()?
    } else {
        vec![0]
    };
    for offset in face_offsets {
        file.seek(SeekFrom::Start(offset))?;
        let mut sfnt = [0; 12];
        file.read_exact(&mut sfnt)?;
        let count = u16::from_be_bytes(sfnt[4..6].try_into().expect("sfnt 头有 12 字节")) as usize;
        let mut tables = vec![0; count * 16];
        file.read_exact(&mut tables)?;
        if tables
            .chunks_exact(16)
            .any(|record| matches!(&record[..4], b"glyf" | b"CFF "))
        {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::Cursor;

    /// 最小 sfnt：12 位元組頭（版本 + 表數 + 三個檢索欄位）+ 只填 tag 的表記錄，偏移與長度本函式不看。
    fn sfnt(version: u32, tags: &[&[u8]]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&version.to_be_bytes());
        out.extend_from_slice(&(tags.len() as u16).to_be_bytes());
        out.extend_from_slice(&[0; 6]);
        for tag in tags {
            out.extend_from_slice(tag);
            out.extend_from_slice(&[0; 12]);
        }
        out
    }

    /// 只有頭的 TTC，面偏移由呼叫方按真實佈局給。
    fn ttc(face_offsets: [u32; 2]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"ttcf");
        out.extend_from_slice(&[0; 4]);
        out.extend_from_slice(&2_u32.to_be_bytes());
        for offset in face_offsets {
            out.extend_from_slice(&offset.to_be_bytes());
        }
        out
    }

    fn rasterizable(bytes: &[u8]) -> bool {
        has_rasterizable_outline(&mut Cursor::new(bytes)).unwrap()
    }

    #[test]
    fn glyf_and_cff_are_rasterizable_hvgl_is_not() {
        assert!(rasterizable(&sfnt(0x0001_0000, &[b"glyf", b"head"])));
        assert!(rasterizable(&sfnt(
            u32::from_be_bytes(*b"OTTO"),
            &[b"CFF ", b"head"]
        )));
        // macOS 26+ 蘋方：可變字型，輪廓在私有的 hvgl 表裡
        assert!(!rasterizable(&sfnt(
            0x0001_0000,
            &[b"hvgl", b"fvar", b"cmap", b"head"]
        )));
    }

    #[test]
    fn ttc_passes_when_any_face_is_rasterizable() {
        let broken = sfnt(0x0001_0000, &[b"hvgl", b"head"]);
        let good = sfnt(0x0001_0000, &[b"glyf", b"head"]);
        let header_len = 12 + 4 * 2;
        let offsets = |second: u32| ttc([(header_len) as u32, second]);

        let mut mixed = offsets((header_len + broken.len()) as u32);
        mixed.extend_from_slice(&broken);
        mixed.extend_from_slice(&good);
        assert!(rasterizable(&mixed));

        let mut all_broken = offsets((header_len + broken.len()) as u32);
        all_broken.extend_from_slice(&broken);
        all_broken.extend_from_slice(&broken);
        assert!(!rasterizable(&all_broken));
    }
}
