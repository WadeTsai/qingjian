//! Rime 碼錶文本的解析：列位置、跨檔案合表與匯入統計。

use std::path::Path;

use super::columns::{Columns, default_column_names, names_to_columns};
use super::table::is_valid_code;

/// 一份碼錶檔案（不含 `import_tables` 合進來的部分）解析出來的東西。
#[derive(Debug, Default)]
pub(super) struct ParsedTable {
    /// YAML 頭裡的 `name:`。
    pub name: Option<String>,

    /// YAML 頭裡的 `version:`。
    pub version: Option<String>,

    /// 要一併讀入的其他碼錶（`import_tables` 的原樣字串，相對路徑由呼叫方按主檔案目錄解析）。
    pub import_tables: Vec<String>,

    /// 列位置。
    pub columns: Columns,

    /// `(詞, 碼)` 對，碼已經過校驗。
    pub pairs: Vec<(String, String)>,

    /// 正文行數（不含註釋與空行）。
    pub read: usize,

    /// 成功建成條目的行數。
    pub with_code: usize,

    /// 有詞沒有碼的行數。
    pub no_code: usize,

    /// 碼非法被丟掉的行數。
    pub skipped: usize,
}

impl ParsedTable {
    /// 解析一份碼錶文本：`---` … `...` 是 YAML 頭（`name` / `version` / `columns` / `import_tables`），
    /// 正文每行按 `columns` 的列位置取詞與碼。沒有頭、直接 `詞\t碼` 的純文本也認（按預設列序）。
    pub fn parse(text: &str) -> Self {
        let mut parsed = Self::default();
        let mut names: Vec<String> = default_column_names();
        let mut in_header = false;
        let mut header_done = false;
        let mut pending_imports = false;
        let mut body: Vec<&str> = Vec::new();
        for raw in text.lines() {
            let line = raw.trim_end();
            let trimmed = line.trim();
            if !header_done {
                if trimmed == "---" {
                    in_header = true;
                    continue;
                }
                if trimmed == "..." {
                    header_done = true;
                    pending_imports = false;
                    continue;
                }
                // 還沒顯式進 `---` 頭：開頭的空行與 `#` 註釋行先跳過——
                // 首行是註釋 / 空行會把下面的啟發式永久置位，整份正文都被當頭吞掉
                if !in_header && (trimmed.is_empty() || trimmed.starts_with('#')) {
                    continue;
                }
                // 頭裡（`---` 之後），或還沒進頭但這一行不像正文（沒有製表符）
                if in_header || !line.contains('\t') {
                    in_header = true;
                    if pending_imports {
                        match trimmed.strip_prefix("- ") {
                            Some(item) => {
                                push_import(&mut parsed, item);
                                continue;
                            }
                            None => pending_imports = false,
                        }
                    }
                    if let Some((key, value)) = split_key(trimmed) {
                        match key {
                            "name" => parsed.name = Some(unquote(value)),
                            "version" => parsed.version = Some(unquote(value)),
                            "columns" => {
                                names = inline_list(value).unwrap_or_else(|| {
                                    value.split_whitespace().map(str::to_owned).collect()
                                })
                            }
                            "import_tables" => match inline_list(value) {
                                Some(items) => parsed.import_tables.extend(items),
                                None => pending_imports = true,
                            },
                            _ => {}
                        }
                    }
                    continue;
                }
                header_done = true;
            }
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            body.push(line);
        }
        parsed.columns = names_to_columns(&names);
        for line in body {
            parsed.push_body(line);
        }
        parsed
    }

    /// 正文一行：按列下標取詞與碼。碼列裡空格分開的多個碼都收（Rime 的簡碼 / 全碼寫法）。
    fn push_body(&mut self, line: &str) {
        self.read += 1;
        let fields: Vec<&str> = line.split('\t').map(str::trim).collect();
        let Some(word) = fields
            .get(self.columns.text)
            .copied()
            .filter(|word| !word.is_empty())
        else {
            self.skipped += 1;
            return;
        };
        let Some(code_index) = self.columns.code else {
            self.no_code += 1;
            return;
        };
        let field = fields.get(code_index).copied().unwrap_or_default();
        let mut produced = 0;
        for token in field.split_whitespace() {
            let code = token.to_ascii_lowercase();
            if is_valid_code(&code) {
                self.pairs.push((word.to_owned(), code));
                produced += 1;
            }
        }
        if produced > 0 {
            self.with_code += 1;
        } else if field.is_empty() {
            self.no_code += 1;
        } else {
            self.skipped += 1;
        }
    }
}

/// 追加一條 `import_tables`。
fn push_import(parsed: &mut ParsedTable, item: &str) {
    let item = unquote(item);
    if !item.is_empty() {
        parsed.import_tables.push(item);
    }
}

/// `key: value` 拆成兩半；不是這個形狀返回 `None`。
fn split_key(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.split_once(':')?;
    let key = key.trim();
    (!key.is_empty() && !key.contains(char::is_whitespace)).then_some((key, value.trim()))
}

/// `[a, b]` 形式的行內列表；不是這個形狀返回 `None`。
fn inline_list(value: &str) -> Option<Vec<String>> {
    let body = value.strip_prefix('[')?.strip_suffix(']')?;
    Some(
        body.split(',')
            .map(|item| unquote(item.trim()))
            .filter(|item| !item.is_empty())
            .collect(),
    )
}

/// 去掉 YAML 值兩邊的引號。
fn unquote(value: &str) -> String {
    value.trim().trim_matches(['\'', '"']).to_owned()
}

/// 碼錶檔案的主幹名：`stroke.dict.yaml` → `stroke`（與詞庫匯入同一套規則）。
pub(super) fn stem_of(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    let stem = crate::import::strip_extensions(name);
    (!stem.is_empty()).then_some(stem)
}
