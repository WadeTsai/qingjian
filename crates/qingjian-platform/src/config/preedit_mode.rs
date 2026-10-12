use serde::{Deserialize, Serialize};

/// 組句中的拼音（preedit）顯示在哪裡。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PreeditMode {
    /// 行內 marked text 與候選視窗頂部都顯示。
    #[default]
    Both,

    /// 只在行內（應用裡的 marked text），候選視窗不帶拼音行。
    Inline,

    /// 只在候選視窗頂部，應用裡不放 marked text（終端、部分 Electron 應用畫不好行內拼音時用）。
    Window,
}

impl PreeditMode {
    /// 全部取值，設定介面按這個順序列出。
    pub const ALL: [Self; 3] = [Self::Both, Self::Inline, Self::Window];

    /// 配置檔案裡的寫法。
    pub fn key(self) -> &'static str {
        match self {
            Self::Both => "both",
            Self::Inline => "inline",
            Self::Window => "window",
        }
    }

    /// 介面上的名字。
    pub fn label(self) -> &'static str {
        match self {
            Self::Both => "行內 + 候選視窗",
            Self::Inline => "只在行內",
            Self::Window => "只在候選視窗",
        }
    }

    /// 要不要往應用裡放 marked text。
    pub fn inline(self) -> bool {
        !matches!(self, Self::Window)
    }

    /// 候選視窗頂部要不要畫拼音行。
    pub fn in_window(self) -> bool {
        !matches!(self, Self::Inline)
    }
}
