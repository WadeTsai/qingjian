//! 「雲服務」頁：本地整句模型開關（`[model]`）、`[predict]` 各項與「測試連線」（後臺執行緒跑）。

use qingjian_predict::{ConnectionTest, PredictConfig};
use windows_reactor::*;

use crate::panel::cloud_status::CloudStatus;
use crate::panel::controls::{field, labeled, note, page};
use crate::panel::{Message, Settings};

/// 後臺跑一次連通性測試，輪詢到有結果或被取消。
pub(crate) fn run_test(
    config: &PredictConfig,
    cancel: &CancellationToken,
) -> Result<String, String> {
    let test = ConnectionTest::start(config).map_err(|error| error.to_string())?;
    loop {
        if cancel.is_cancelled() {
            return Err("已取消".to_owned());
        }
        if let Some(result) = test.poll() {
            return result
                .map(|report| {
                    format!(
                        "連線成功：模型 {}，耗時 {} ms",
                        report.model,
                        report.elapsed.as_millis()
                    )
                })
                .map_err(|error| error.to_string());
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

pub(crate) fn view(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    let p = &settings.config.predict;
    let status = match &settings.cloud_status {
        CloudStatus::Idle => String::new(),
        CloudStatus::Testing => "測試中…".to_owned(),
        CloudStatus::Ok(message) => message.clone(),
        CloudStatus::Failed(message) => format!("失敗：{message}"),
    };
    let rows = [
        field(
            "本地整句模型",
            "隨包的小模型在本機給整句候選重新排序，全程離線；停鍵後幾十毫秒生效。關掉只用詞庫統計。",
            ToggleSwitch::new()
                .is_on(settings.config.model.enabled)
                .on_toggled(context.callback(Message::LocalModel)),
        ),
        field(
            "啟用雲聯想",
            "開啟後組句時會把游標附近的幾十個字發給下面的服務，讓模型補全整句、聯想下文；金鑰框裡的內容不傳送。",
            ToggleSwitch::new()
                .is_on(p.enabled)
                .on_toggled(context.callback(Message::CloudEnabled)),
        ),
        field(
            "雲端詞格數",
            "雲端詞到了只補進第一頁末尾這幾格，前面的本地候選不動；沒到就什麼都不變。0 = 只要整句補全。",
            NumberBox::new()
                .minimum(0.0)
                .maximum(9.0)
                .value(p.slots as f64)
                .on_value_changed(context.callback(Message::CloudSlots)),
        ),
        field(
            "整句補全",
            "preedit 右側給出整句補全，按 Tab 採用。",
            ToggleSwitch::new()
                .is_on(p.sentence)
                .on_toggled(context.callback(Message::CloudSentence)),
        ),
        field(
            "介面地址",
            "",
            TextBox::new()
                .text(p.base_url.clone())
                .on_text_changed(context.callback(Message::CloudBaseUrl)),
        ),
        field(
            "模型",
            "",
            TextBox::new()
                .text(p.model.clone())
                .on_text_changed(context.callback(Message::CloudModel)),
        ),
        field(
            "API 金鑰",
            "只儲存在這臺電腦上，不會隨配置檔案匯出，也不顯示已填的值。留空則讀環境變數 QINGJIAN_API_KEY。",
            PasswordBox::new()
                .password(p.api_key.clone().unwrap_or_default())
                .placeholder_text("留空則讀環境變數 QINGJIAN_API_KEY")
                .on_password_changed(context.callback(Message::CloudApiKey)),
        ),
        labeled(
            "",
            StackPanel::new()
                .orientation(Orientation::Horizontal)
                .spacing(12.0)
                .children((
                    Button::new()
                        .on_click(context.message(Message::TestConnection))
                        .content("測試連線"),
                    TextBlock::new().text(status),
                )),
        ),
        note(
            "用上面填的地址、模型、金鑰發一條最小請求。走不通時先查這裡；Server 程序看不到終端裡的代理變數。",
        ),
    ];
    page("雲服務", StackPanel::new().spacing(16.0).children(rows))
}
