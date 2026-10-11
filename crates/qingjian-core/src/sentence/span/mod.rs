//! 詞圖的跨度：一段音節位置上查到的詞（`SpanWord`）與按輸入串快取的查詞結果（`SpanCache`）。

mod cache;
mod word;

pub use cache::{MAX_SPAN_CACHE_ENTRIES, SpanCache};
pub use word::SpanWord;
