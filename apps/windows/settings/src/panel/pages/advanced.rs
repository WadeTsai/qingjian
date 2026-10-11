//! 「進階」頁：開啟配置檔案 / 資料目錄 / 日誌目錄、詳細日誌、學習開關、輸入日誌。

use qingjian_platform::LogLevel;
use windows_reactor::*;

use crate::panel::controls::{field, note, page};
use crate::panel::{Message, Settings};

pub(crate) fn view(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    let g = &settings.config.general;
    let rows = [
        note("設定改完會自動生效（Server 每秒看一次配置檔案）。只有換學習語言要重啟 Server。"),
        field(
            "配置檔案",
            "",
            Button::new()
                .on_click(context.message(Message::OpenConfigFile))
                .content("在記事本中開啟"),
        ),
        field(
            "資料目錄",
            "",
            Button::new()
                .on_click(context.message(Message::OpenDataDir))
                .content("開啟資料目錄"),
        ),
        field(
            "日誌目錄",
            "輸入法、引擎與設定程式的日誌都在這一個目錄，按天分檔案，保留 7 天。",
            StackPanel::new()
                .orientation(Orientation::Horizontal)
                .spacing(8.0)
                .children((
                    Button::new()
                        .on_click(context.message(Message::OpenLogDir))
                        .content("開啟日誌目錄"),
                    Button::new()
                        .on_click(context.message(Message::ExportLogs))
                        .content("打包日誌到桌面"),
                )),
        ),
        field(
            "詳細日誌",
            "排查問題時臨時開啟，會記下敲的拼音與上屏文字。",
            ToggleSwitch::new()
                .is_on(g.log_level == LogLevel::Debug)
                .on_toggled(context.callback(Message::VerboseLog)),
        ),
        field(
            "學習輸入習慣",
            "按你的選擇調整候選順序、記新詞與敲錯糾正。關掉後不再學，已學的仍參與排序；學習資料在資料目錄裡。",
            ToggleSwitch::new()
                .is_on(g.learning)
                .on_toggled(context.callback(Message::Learning)),
        ),
        field(
            "記錄輸入日誌",
            "每次上屏記一行，只寫本機、不上傳，用於離線評測與個人模型。",
            ToggleSwitch::new()
                .is_on(g.input_log)
                .on_toggled(context.callback(Message::InputLog)),
        ),
        field(
            "清空輸入日誌",
            "",
            Button::new()
                .on_click(context.message(Message::ClearInputLog))
                .content("清空輸入日誌"),
        ),
    ];
    page("進階", StackPanel::new().spacing(16.0).children(rows))
}
