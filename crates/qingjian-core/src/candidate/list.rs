use serde::{Deserialize, Serialize};

use super::Candidate;

/// 排好序的候選列表，平臺層按順序繪製。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateList {
    /// 候選，索引 0 為首選。
    pub items: Vec<Candidate>,
}
