//! 已凍結拼音與來源的字詞樣本；答案只用於結果歸檔，不參與候選構造。

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Entry {
    pub id: String,

    pub text: String,

    pub keys: String,

    pub source: String,

    pub category: String,
}
