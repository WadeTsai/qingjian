//! 按鍵轉發給 Server 之後要做的事：Server 互動在引擎借用裡做完，放掉借用再按這個列舉走編輯會話。

/// 一次按鍵轉發給 Server 後、放掉引擎借用要做的事。
pub(super) enum Next {
    /// 把上屏文本 / 組句拼音行寫進文件。
    Document {
        /// 本次要立即上屏的文本。
        commit: Option<String>,

        /// 組句拼音行；空串表示收組句。
        preedit: String,

        /// 這個鍵吃不吃。
        consumed: bool,
    },

    /// 「翻譯選中文字」：起非同步只讀會話讀當前選區。
    ReadSelection {
        /// 請求標識，回給 Server 對上是哪一次詢問。
        request: u64,
    },

    /// 轉發出錯、已斷連：放行本鍵。
    Abort,
}
