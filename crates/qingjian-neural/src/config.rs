use std::path::Path;

use serde::Deserialize;

use crate::NeuralError;

/// 模型結構，來自匯出目錄的 `config.json`（訓練指令碼 `common.py::ModelConfig` 原樣寫出）。
#[derive(Debug, Clone, Deserialize)]
pub struct ModelConfig {
    /// 字表大小。
    pub vocab_size: usize,

    /// 層數。
    pub n_layer: usize,

    /// 隱層寬度。
    pub n_embd: usize,

    /// 注意力頭數。
    pub n_head: usize,

    /// 最長上下文（token 數），位置嵌入的行數。
    pub context: usize,
}

impl ModelConfig {
    /// 解析 `config.json` 的正文；`path` 只用來報錯。
    pub(crate) fn from_json(text: &str, path: &Path) -> Result<Self, NeuralError> {
        serde_json::from_str(text).map_err(|source| NeuralError::Json {
            path: path.to_owned(),
            source,
        })
    }
}
