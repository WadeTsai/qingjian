//! 候選視窗的繪製方式（青簡渲染器 / 系統原生），對應配置項 `candidate_renderer`。

use serde::{Deserialize, Serialize};

/// 候選視窗由誰繪製：青簡自己的渲染器（各平臺一致，主題走它）還是系統的原生繪製。
/// 原生繪製是過渡期的退路：渲染器有問題時使用者能切回去繼續用，穩定一個版本後刪掉。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CandidateRenderer {
    /// 青簡渲染器出點陣圖，殼只貼圖。
    #[default]
    Qingjian,

    /// 平臺原生繪製（macOS AppKit / Windows GDI）。
    System,
}

impl CandidateRenderer {
    /// 全部取值，設定介面按這個順序列出。
    pub const ALL: [Self; 2] = [Self::Qingjian, Self::System];

    /// 配置檔案裡的寫法。
    pub fn key(self) -> &'static str {
        match self {
            Self::Qingjian => "qingjian",
            Self::System => "system",
        }
    }

    /// 介面上的名字。
    pub fn label(self) -> &'static str {
        match self {
            Self::Qingjian => "青简渲染器",
            Self::System => "系统绘制",
        }
    }
}
