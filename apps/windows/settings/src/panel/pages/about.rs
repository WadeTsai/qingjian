//! 「關於」頁：版本、檢查更新、許可證、隨包資料的來源與署名（第三方許可要求署名在分發物裡可見）、隱私與反饋。

use qingjian_platform::UpdateChannel;
use qingjian_update::Checker;
use windows_reactor::*;

use crate::panel::controls::{field, note, page};
use crate::panel::{Message, Settings};

/// QINGJIAN_VERSION 由 build.rs 給：-dev 版接 git 短雜湊。
pub(crate) const VERSION: &str = env!("QINGJIAN_VERSION");

pub(crate) const WEBSITE_URL: &str = "https://qingjian.app";

pub(crate) const REPOSITORY_URL: &str = "https://github.com/qingjian-team";

/// 與倉庫根 `LICENSE` 一致。
const LICENSE_NOTE: &str = "自由軟體，GPL-3.0-or-later 許可證：可以自由使用、修改與再分發，修改後分發須同樣開源。官方渠道免費。";

/// 與 macOS「關於」頁一致。
const ATTRIBUTIONS: &[(&str, &str)] = &[
    (
        "詞庫",
        "通用規範漢字表；現代漢語常用詞表（liuxilu 校對版）；THUOCL（清華大學自然語言處理實驗室，MIT）；讀音取自 Unihan（Unicode License v3）。",
    ),
    (
        "語言模型",
        "中文維基百科（CC BY-SA 4.0）與 LCCC（清華大學 CoAI，MIT）語料統計。",
    ),
    ("釋義表", "由大語言模型（DeepSeek）生成，青簡自建。"),
    ("emoji", "Unicode CLDR annotations（Unicode License v3）。"),
    (
        "英文詞表",
        "ESDB / SCOWL（© Kevin Atkinson，按其許可保留版權宣告）；CSpell 詞典（MIT）。",
    ),
    (
        "詞彙等級",
        "CEFR-J Wordlist v1.5（Yukio Tono，cefr-j.org）；Octanove Vocabulary Profile C1/C2（CC BY-SA 4.0）；JLPT 詞表（tanos.co.uk，CC BY）。",
    ),
    (
        "五筆碼錶",
        "86 五筆極點碼錶（sxjudya/rime-wubi86-jidian，Apache-2.0）；編碼來自上游，詞頻由青簡詞庫按詞面回填。",
    ),
    (
        "筆畫碼錶",
        "CNS11643 全字型檔筆順資料（數位發展部，政府資料開放授權條款第 1 版 / OFL-1.1）；大陸筆順按通用規範字表校正。",
    ),
];

const PRIVACY_NOTE: &str = "青簡不上傳任何資料。開著「自動檢查更新」時每天向官網讀一次版本列表，請求不帶任何標識，上面可以關。開著雲聯想時，游標附近的文字與拼音會發給你在「雲服務」頁填的 AI 服務商（預設 DeepSeek）的伺服器，不經過作者。「進階」頁的輸入日誌只寫在本機，可以關掉或清空。";

const FEEDBACK_NOTE: &str = "遇到問題點「打包日誌到桌面」，把生成的 zip 發給作者即可（含三個程序的日誌與配置檔案，不含金鑰）。預設日誌不含你敲的內容；排查排序問題時作者可能請你在「進階」頁臨時開啟詳細日誌。";

/// 「立即檢查」旁邊那行字。
fn update_status(settings: &Settings) -> String {
    if settings.update_checking {
        return "正在檢查…".to_owned();
    }
    if let Some(error) = &settings.update_error {
        return format!("檢查失敗：{error}");
    }
    let update = &settings.config.update;
    match Checker::available_in(&settings.update_state, VERSION, update) {
        Some(found) => format!("有新版本 {}", found.version),
        None if Checker::is_dev_version(VERSION) => "本地開發版，不檢查更新".to_owned(),
        None if settings.update_state.checked_at > 0 => "已是最新版本".to_owned(),
        None => String::new(),
    }
}

pub(crate) fn view(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    let update = &settings.config.update;
    let available = Checker::available_in(&settings.update_state, VERSION, update).is_some();
    let mut update_buttons: Vec<View> = vec![
        Button::new()
            .on_click(context.message(Message::CheckUpdateNow))
            .content("立即檢查"),
    ];
    if available {
        update_buttons.push(
            Button::new()
                .on_click(context.message(Message::OpenDownload))
                .content("下載新版"),
        );
    }
    update_buttons.push(note(&update_status(settings)));
    let mut attributions: Vec<View> = Vec::with_capacity(ATTRIBUTIONS.len());
    for (name, text) in ATTRIBUTIONS {
        attributions.push(note(&format!("{name}：{text}")));
    }
    let body = StackPanel::new().spacing(12.0).children([
        TextBlock::new()
            .text(format!("青簡 Windows {VERSION}"))
            .font_size(16.0)
            .font_weight(FontWeight::SEMI_BOLD)
            .into(),
        // QINGJIAN_BUILD 由 build.rs 從 git 取
        note(&format!(
            "構建 {}",
            option_env!("QINGJIAN_BUILD").unwrap_or("本地構建")
        )),
        StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(12.0)
            .children((
                Button::new()
                    .on_click(context.message(Message::OpenWebsite))
                    .content("官網"),
                Button::new()
                    .on_click(context.message(Message::OpenRepository))
                    .content("GitHub"),
                Button::new()
                    .on_click(context.message(Message::OpenDataDir))
                    .content("開啟資料目錄"),
                Button::new()
                    .on_click(context.message(Message::OpenLogDir))
                    .content("開啟日誌目錄"),
                Button::new()
                    .on_click(context.message(Message::ExportLogs))
                    .content("打包日誌到桌面"),
            )),
        note(LICENSE_NOTE),
        field(
            "自動檢查更新",
            "每天向官網讀一次版本列表，有新版在這裡提示，不會自動下載安裝。",
            ToggleSwitch::new()
                .is_on(update.check)
                .on_toggled(context.callback(Message::UpdateCheck)),
        ),
        field(
            "更新渠道",
            "測試版渠道還會提示 alpha / beta / rc 版本。",
            ComboBox::new()
                .items_source(UpdateChannel::ALL.iter().map(|channel| channel.label()))
                .selected_index(
                    UpdateChannel::ALL
                        .iter()
                        .position(|channel| *channel == update.channel)
                        .unwrap_or(0),
                )
                .on_selection_changed(context.callback(Message::UpdateChannel)),
        ),
        StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(12.0)
            .keyed_children(
                update_buttons
                    .into_iter()
                    .enumerate()
                    .map(|(index, view)| KeyedView::new(format!("update-{index}"), view)),
            ),
        TextBlock::new()
            .text("資料來源與署名")
            .font_weight(FontWeight::SEMI_BOLD)
            .into(),
        StackPanel::new().spacing(6.0).keyed_children(
            attributions
                .into_iter()
                .enumerate()
                .map(|(index, view)| KeyedView::new(index.to_string(), view)),
        ),
        note(PRIVACY_NOTE),
        note(FEEDBACK_NOTE),
    ]);
    page("關於", body)
}
