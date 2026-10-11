//! 雲端：測試連線的起停與輪詢，聯想的等待 / 取消 / 輪詢與結果套用到當前頁。

mod predict_monitor;
mod test_monitor;

pub(super) use predict_monitor::PredictMonitor;
pub(super) use test_monitor::CloudTestMonitor;

use super::*;

impl Host {
    /// 清空輸入日誌檔案；開著的話重新開啟繼續記。
    /// 「測試連線」：用當前配置發一條最小請求，結果回到偏好設定視窗底部的狀態行。
    pub(super) fn start_cloud_test(&mut self) {
        let config = self.settings.config().predict.clone();
        match ConnectionTest::start(&config) {
            Ok(test) => {
                self.cloud_test = Some(test);
                self.cloud_test_monitor.start();
                self.preferences.set_status(&format!(
                    "正在测试云服务连接：{} · {} …",
                    config.base_url, config.model
                ));
            }
            Err(error) => {
                self.preferences.set_status(&format!(
                    "云服务连接测试没能开始：{}",
                    describe_predict_error(&error)
                ));
            }
        }
    }

    /// 輪詢定時器每 0.2 秒來一次：結果到了就顯示並停表；等太久也停。
    pub fn poll_cloud_test(&mut self) {
        let Some(test) = &self.cloud_test else {
            self.cloud_test_monitor.stop();
            return;
        };
        let Some(outcome) = test.poll() else {
            if self.cloud_test_monitor.expired() {
                self.cloud_test = None;
                self.cloud_test_monitor.stop();
                self.preferences
                    .set_status("云服务连接测试：等了 30 秒没有结果，请检查网络或代理");
            }
            return;
        };
        self.cloud_test = None;
        self.cloud_test_monitor.stop();
        match outcome {
            Ok(report) => self.preferences.set_status(&format!(
                "云服务连接正常：{} 在 {} ms 内回复",
                report.model,
                report.elapsed.as_millis()
            )),
            Err(error) => self.preferences.set_status(&format!(
                "云服务连接失败：{}",
                describe_predict_error(&error)
            )),
        }
    }

    /// 已發出聯想請求：清掉舊結果，開始輪詢。
    pub fn await_prediction(&mut self) {
        self.sentence = None;
        self.monitor.start();
    }

    /// 作廢聯想：不再輪詢，丟掉已到的整句。
    pub fn cancel_prediction(&mut self) {
        self.engine.cancel_prediction();
        self.monitor.stop();
        self.sentence = None;
    }

    /// 定時器回撥：結果到了就畫上去。雲端詞補進第一頁末尾，整句掛在 preedit 右側。
    pub fn poll_prediction(&mut self) {
        let Some(prediction) = self.engine.poll_prediction() else {
            if self.monitor.expired() {
                self.monitor.stop();
            }
            return;
        };
        self.monitor.stop();
        self.apply_prediction(prediction);
    }

    pub(super) fn apply_prediction(&mut self, prediction: Prediction) {
        // 翻譯選中文字：譯文作為唯一候選擺進視窗，等使用者回車替換或 Esc 放棄
        if let Some(job) = self.translation.as_mut() {
            match prediction.sentence {
                Some(text) => {
                    job.result = Some(text.clone());
                    self.reset_session(None, vec![cloud_candidate(text)]);
                    self.render();
                }
                None => {
                    tracing::info!("云端没有给出译文");
                    self.end_translation();
                }
            }
            return;
        }
        if self.engine.composition().is_empty() {
            return;
        }
        // 雲端詞補進第一頁末尾（前面的本地候選不動），整句掛在 preedit 右側；雲端詞沒有譯文，先補上。
        // 使用者已經翻到後面的頁、或高亮已經移到會被挪走的那幾格時不補：那幾格正被他看著 / 要選
        let layout = &self.session.layout;
        let untouched = cloud_slots_untouched(
            self.session.page,
            self.session.highlighted,
            layout.local().is_empty(),
            layout.page_size(),
            layout.capacity(),
        );
        if untouched && !prediction.words.is_empty() {
            let mut words = qingjian_core::CandidateList {
                items: prediction
                    .words
                    .into_iter()
                    .map(CloudWord::into_candidate)
                    .collect(),
            };
            self.engine.annotate(&mut words);
            let filled = self.session.layout.set_cloud(words.items);
            tracing::debug!(filled, "云端词已补进候选");
        }
        self.sentence = prediction.sentence;
        self.render();
    }
}

/// 用系統的 `open` 開啟檔案或目錄；輸入法程序沒有自己的文件視窗，交給訪達 / 預設編輯器最省事。
/// 雲端詞到了還能不能補進第一頁：使用者還在第一頁，且高亮沒落在會被雲端詞頂掉的那幾格上。
/// 沒有本地候選（問字模式）時整頁都是雲端的，談不上挪走誰，永遠能補——
/// 之前沒分這種情況，`page_size - capacity` 算出 0，問字的答案全被當成「會打擾使用者」丟掉了。
pub(super) fn cloud_slots_untouched(
    page: usize,
    highlighted: usize,
    local_empty: bool,
    page_size: usize,
    capacity: usize,
) -> bool {
    page == 0 && (local_empty || highlighted < page_size.saturating_sub(capacity))
}

/// 連通性測試的錯誤換成給使用者看的中文；介面返回的原話保留，方便對著服務商文件查。
pub(super) fn describe_predict_error(error: &PredictError) -> String {
    match error {
        PredictError::MissingApiKey(env) => {
            format!("没有 API 密钥。在上面填一个，或设置环境变量 {env}")
        }
        PredictError::Timeout(ms) => {
            format!("{ms} ms 内没有回复。检查网络或代理：输入法进程不读终端里的代理变量")
        }
        PredictError::EmptyReply => "接口通了但没有返回正文，检查模型名是否正确".to_owned(),
        PredictError::BudgetExhausted => {
            "接口通了但正文是空的：模型开着思考，把输出额度用光了".to_owned()
        }
        PredictError::Encode(error) => format!("请求编码失败：{error}"),
        PredictError::Runtime(error) => format!("起不了后台线程：{error}"),
        PredictError::Api(error) => format!("请求失败：{error}"),
    }
}

/// 翻譯窗口裡的一行：譯文（或佔位文字）當作雲端來源的候選畫出來。
pub(super) fn cloud_candidate(text: String) -> Candidate {
    Candidate {
        text,
        kind: CandidateKind::Cloud,
        syllables: Vec::new(),
        reading: None,
        translation: None,
        aux_code: None,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn question_answers_fill_the_page_even_though_every_slot_is_cloud() {
        // 問字：沒有本地候選，capacity == page_size
        assert!(super::cloud_slots_untouched(0, 0, true, 9, 9));
        // 組句：高亮在前面的本地格上能補，高亮已在末尾兩格（雲端要佔的位置）不補，翻頁後不補
        assert!(super::cloud_slots_untouched(0, 0, false, 9, 2));
        assert!(!super::cloud_slots_untouched(0, 7, false, 9, 2));
        assert!(!super::cloud_slots_untouched(1, 0, false, 9, 2));
    }
}
