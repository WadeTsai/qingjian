//! Rime `.dict.yaml`：`---` … `...` 的 YAML 頭之後每行 `詞\t拼音[\t權重]`，`#` 為註釋。拼音已經是空格分隔的音節。

/// 解析結果：青簡 TSV 文本與 YAML 頭裡的名字。
pub struct Parsed {
    pub tsv: String,

    pub name: Option<String>,
}

/// 有 YAML 頭，或第一條非註釋行是 `name:` / `---` 就當 Rime 詞庫。
pub fn looks_like_rime(text: &str) -> bool {
    text.lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with('#'))
        .is_some_and(|l| l == "---" || l.starts_with("name:"))
}

pub fn to_tsv(text: &str) -> Parsed {
    let mut tsv = String::with_capacity(text.len());
    let mut name = None;
    let mut in_header = false;
    let mut header_done = false;
    for raw in text.lines() {
        let line = raw.trim_end();
        if !header_done {
            if line.trim() == "---" {
                in_header = true;
                continue;
            }
            if line.trim() == "..." {
                header_done = true;
                continue;
            }
            if in_header || !line.contains('\t') {
                if let Some(value) = line.trim().strip_prefix("name:") {
                    name = Some(value.trim().trim_matches('"').to_owned());
                }
                continue;
            }
            header_done = true;
        }
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut fields = line.split('\t');
        let (Some(word), Some(pinyin)) = (fields.next(), fields.next()) else {
            continue;
        };
        let weight = fields
            .next()
            .map(str::trim)
            .filter(|w| !w.is_empty())
            .unwrap_or("1");
        // 權重不是整數（Rime 允許 1e5 之類）就當 1
        let weight = weight
            .parse::<u32>()
            .map_or("1".to_owned(), |w| w.to_string());
        tsv.push_str(word.trim());
        tsv.push('\t');
        tsv.push_str(pinyin.trim());
        tsv.push('\t');
        tsv.push_str(&weight);
        tsv.push('\n');
    }
    Parsed { tsv, name }
}
