//! TSV → `.qj`：解析成記憶體結構後原樣落盤，加上後設資料；`model` 是三件套目錄 → `.qjm`，
//! `codes` 是唯一帶計算的一種（筆畫表 + 詞庫 → 碼錶，見 `codes` 模組）。

use std::path::{Path, PathBuf};
use std::time::Instant;

use qingjian_core::Language;
use qingjian_dictionary::Dictionary;
use qingjian_format::Metadata;
use qingjian_lm::BigramModel;
use qingjian_translate::Glossary;

use crate::args::PackKind;
use crate::error::ConvertError;

/// 隨包筆畫碼錶的名稱（`pack codes` 的後設資料預設值）。
const CODES_NAME: &str = "笔画";

/// 許可：CNS11643 筆順資料開放資料「政府資料開放授權條款第 1 版 / OFL-1.1」二選一，取 OFL-1.1。
const CODES_LICENSE: &str = "OFL-1.1";

/// 署名：與 `assets/stroke/README.md` 的「隨包時的許可合規」一致。
const CODES_ATTRIBUTION: &str = "CNS11643 全字庫筆順資料（數位發展部）";

/// 來源：全字型檔開放資料的資料集頁。
const CODES_SOURCE: &str = "https://data.gov.tw/dataset/5961";

/// `pack codes` 的三個路徑（別的種類不給）。都不給時都從 `out_dir` 裡找。
#[derive(Debug, Default)]
pub struct CodePaths<'a> {
    /// 筆畫表（`stroke` 子命令的產物）。
    pub stroke: Option<&'a Path>,

    /// 取碼用的詞庫。
    pub dict: Option<&'a Path>,

    /// 碼錶產物。
    pub output: Option<&'a Path>,
}

/// 打包一種資料。`inputs` 為空時從 `out_dir` 裡找預設的 TSV。
pub fn pack(
    kind: PackKind,
    inputs: &[PathBuf],
    paths: &CodePaths<'_>,
    language: &str,
    metadata: Metadata,
    out_dir: &Path,
) -> Result<(), ConvertError> {
    let metadata = Metadata {
        generator: format!("qingjian-dict-convert {}", env!("CARGO_PKG_VERSION")),
        ..metadata
    };
    // 只有 codes 自己帶後設資料預設值（名稱、許可、署名都是產品決定），別的種類仍然要顯式給。
    if kind != PackKind::Codes && metadata.name.is_empty() {
        return Err(ConvertError::MissingName { kind: kind.name() });
    }
    let started = Instant::now();
    match kind {
        PackKind::Dict => {
            let input = inputs
                .first()
                .cloned()
                .unwrap_or_else(|| out_dir.join("dict.tsv"));
            let dictionary = Dictionary::from_path(&input)?;
            let out = out_dir.join("dict.qj");
            dictionary.write_qj(&out, &metadata)?;
            report(&out, dictionary.len(), started);
        }
        PackKind::Lm => {
            let (unigram, bigram) = match inputs {
                [unigram, bigram, ..] => (unigram.clone(), bigram.clone()),
                _ => (
                    out_dir.join("lm-unigram.tsv"),
                    out_dir.join("lm-bigram.tsv"),
                ),
            };
            let model = BigramModel::from_paths(&unigram, &bigram)?;
            let out = out_dir.join("lm.qj");
            model.write_qj(&out, &metadata)?;
            report(&out, model.bigram_count(), started);
        }
        PackKind::Glossary => {
            let language: Language = language.parse().map_err(|_| ConvertError::Format {
                path: PathBuf::from(language),
                line: 0,
                reason: "language must be en / ja / zh / es".to_owned(),
            })?;
            let input = inputs.first().cloned().unwrap_or_else(|| {
                PathBuf::from("assets/glossary").join(format!("glossary-{}.tsv", language.code()))
            });
            let glossary = Glossary::from_path(language, &input)?;
            let out = out_dir.join(format!("glossary-{}.qj", language.code()));
            glossary.write_qj(&out, &metadata)?;
            report(&out, glossary.len(), started);
        }
        PackKind::Model => {
            let input = inputs
                .first()
                .cloned()
                .unwrap_or_else(|| PathBuf::from("data/models/hanzhang-zhiwei"));
            let out = paths
                .output
                .map(Path::to_path_buf)
                .unwrap_or_else(|| out_dir.join("model.qjm"));
            let parameters = qingjian_neural::qjm::pack(&input, &out, &metadata)?;
            report(
                &out,
                usize::try_from(parameters).unwrap_or(usize::MAX),
                started,
            );
        }
        PackKind::Codes => {
            let stroke = paths
                .stroke
                .map(Path::to_path_buf)
                .unwrap_or_else(|| out_dir.join("codes").join("stroke.tsv"));
            let dict = paths
                .dict
                .map(Path::to_path_buf)
                .unwrap_or_else(|| out_dir.join("dict.qj"));
            let out = paths
                .output
                .map(Path::to_path_buf)
                .unwrap_or_else(|| out_dir.join("codes").join("stroke.qj"));
            crate::codes::build(&stroke, &dict, &out, &codes_metadata(metadata, &stroke))?;
        }
    }
    Ok(())
}

/// `pack codes` 的後設資料預設值：名稱、許可、署名、來源都是這份產品資料的決定
/// （許可依據見 `docs/design/aux-code.md` 的「許可」一節），呼叫方不用重複給，給了的以給的為準。
fn codes_metadata(mut metadata: Metadata, stroke: &Path) -> Metadata {
    if metadata.name.is_empty() {
        metadata.name = CODES_NAME.to_owned();
    }
    if metadata.license.is_empty() {
        metadata.license = CODES_LICENSE.to_owned();
    }
    if metadata.attribution.is_empty() {
        metadata.attribution = CODES_ATTRIBUTION.to_owned();
    }
    if metadata.source.is_empty() {
        metadata.source = CODES_SOURCE.to_owned();
    }
    if metadata.version.is_empty() {
        // 上游沒有版本號：取筆畫表（CNS 筆順資料的快照）的日期當資料版本。
        metadata.version = source_date(stroke)
            .map(|date| date.to_string())
            .unwrap_or_default();
    }
    metadata
}

/// 檔案的日期（本地時區）；拿不到時 `None`。
fn source_date(path: &Path) -> Option<jiff::civil::Date> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    let seconds = modified
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    let stamp = jiff::Timestamp::from_second(i64::try_from(seconds).ok()?).ok()?;
    Some(stamp.to_zoned(jiff::tz::TimeZone::system()).date())
}

fn report(out: &Path, entries: usize, started: Instant) {
    let size = std::fs::metadata(out).map(|m| m.len()).unwrap_or(0);
    tracing::info!(
        out = %out.display(),
        entries,
        size_mb = size / 1_000_000,
        elapsed_ms = started.elapsed().as_millis(),
        "已打包"
    );
}
