use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

use crate::NeuralError;

/// 未登入字元的 token。
pub const UNK: u32 = 1;

/// 句首 / 行分隔 token（訓練時每行末尾補它，打分時當句首用）。
pub const EOS: u32 = 2;

#[derive(Deserialize)]
struct VocabFile {
    tokens: Vec<String>,
}

/// 字級字表：一個 Unicode 字元一個 token，開頭幾個是 `<...>` 形式的特殊 token。
#[derive(Debug, Clone)]
pub struct Vocab {
    /// 字元 → token。
    index: HashMap<char, u32>,

    /// token → 字面，解碼生成結果用。
    tokens: Vec<String>,

    /// P2C 字表裡分開拼音與漢字兩段的 token；字級 LM 的字表沒有。
    sep: Option<u32>,
}

impl Vocab {
    pub fn load(path: &Path) -> Result<Self, NeuralError> {
        let text = std::fs::read_to_string(path).map_err(|source| NeuralError::Io {
            path: path.to_owned(),
            source,
        })?;
        Self::from_json(&text, path)
    }

    /// 解析 `vocab.json` 的正文；`path` 只用來報錯。
    pub(crate) fn from_json(text: &str, path: &Path) -> Result<Self, NeuralError> {
        let file: VocabFile = serde_json::from_str(text).map_err(|source| NeuralError::Json {
            path: path.to_owned(),
            source,
        })?;
        // 特殊 token 靠「不是單個字元」認，不寫死個數：字級 LM 的字表有 3 個，P2C 的多一個 `<sep>`。
        let mut index = HashMap::with_capacity(file.tokens.len());
        let mut sep = None;
        for (i, token) in file.tokens.iter().enumerate() {
            let mut chars = token.chars();
            match (chars.next(), chars.next()) {
                (Some(ch), None) => {
                    index.insert(ch, i as u32);
                }
                (Some(_), Some(_)) => {
                    if token == "<sep>" {
                        sep = Some(i as u32);
                    }
                }
                (None, _) => return Err(NeuralError::Corrupt("empty vocab token")),
            }
        }
        Ok(Self {
            index,
            tokens: file.tokens,
            sep,
        })
    }

    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    /// P2C 的 `<sep>`；不是 P2C 字表就沒有。
    pub fn sep(&self) -> Option<u32> {
        self.sep
    }

    /// token 序列 → 文本；越界的 token 跳過。
    pub fn decode(&self, ids: &[u32]) -> String {
        ids.iter()
            .filter_map(|&id| self.tokens.get(id as usize))
            .map(String::as_str)
            .collect()
    }

    /// 逐字元編碼，不認識的記 [`UNK`]。
    pub fn encode(&self, text: &str) -> Vec<u32> {
        text.chars()
            .map(|c| self.index.get(&c).copied().unwrap_or(UNK))
            .collect()
    }
}
