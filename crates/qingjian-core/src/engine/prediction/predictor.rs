use super::policy::PredictionPolicy;
use super::request::PredictionRequest;
use super::response::Prediction;

/// 聯想提供方。實現可以聯網，但介面是**非阻塞**的：提交立刻返回，結果由殼定時輪詢。
///
/// 「最新請求優先」：實現收到新請求時可以直接丟掉還沒發出去的舊請求；
/// Engine 只認序號等於最近一次提交的結果，其餘一律丟棄。
pub trait Predictor: Send {
    /// 觀察視窗、條數上限與開關。
    fn policy(&self) -> PredictionPolicy;

    /// 提交請求，不能阻塞。
    fn submit(&mut self, request: PredictionRequest);

    /// 取一條已到達的結果，沒有就返回 `None`，不能阻塞。
    fn poll(&mut self) -> Option<Prediction>;

    /// 是否真的會聯想；`false` 時 Engine 根本不構造請求。
    fn is_enabled(&self) -> bool {
        true
    }
}

/// 不聯想。
#[derive(Debug, Clone, Copy, Default)]
pub struct NoPredictor;

impl Predictor for NoPredictor {
    fn policy(&self) -> PredictionPolicy {
        PredictionPolicy::default()
    }

    fn submit(&mut self, _request: PredictionRequest) {}

    fn poll(&mut self) -> Option<Prediction> {
        None
    }

    fn is_enabled(&self) -> bool {
        false
    }
}
