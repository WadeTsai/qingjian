//! 青簡 macOS 輸入法殼（IMK）。
//!
//! 按鍵進 [`qingjian_core::Engine`]，候選畫在自繪 NSPanel 裡。數字選詞，空格上首選，
//! 回車上屏拼音本身，退格刪一個，Esc 清空，上下鍵移動高亮。
//!
//! 無法 `cargo run`：必須打包成 `.app` 裝到 `~/Library/Input Methods/`，見 `scripts/bundle.sh`。

mod app;
mod candidates;
mod error;
mod host;
mod imk;
mod menubar;
mod preferences;

use objc2::{AnyThread, ClassType, MainThreadMarker};
use objc2_app_kit::NSApplication;
use objc2_foundation::NSString;
use objc2_input_method_kit::IMKServer;

fn main() {
    // `--register` 起的子程序：在乾淨的 TIS 快取裡回讀啟用狀態並切成當前（見 input_source.rs）
    let mut arguments = std::env::args().skip(1);
    if arguments.next().as_deref() == Some(app::input_source::FINISH_FLAG) {
        let source_id = arguments.next().unwrap_or_default();
        std::process::exit(app::input_source::finish_register(&source_id));
    }
    // 主題投稿用：把主題目錄畫成預覽圖寫進它的 preview/，不起 IMK（主題倉庫的 tools/preview.py 調它）
    let mut arguments = std::env::args().skip(1);
    if arguments.next().as_deref() == Some(qingjian_render::preview::FLAG) {
        let Some(dir) = arguments.next() else {
            eprintln!("用法：{} <主题目录>", qingjian_render::preview::FLAG);
            std::process::exit(2);
        };
        std::process::exit(qingjian_render::preview::run(
            std::path::Path::new(&dir),
            candidates::family_files,
        ));
    }
    // 安裝器的 postinstall 以登入使用者身份調 `--register`：註冊、啟用並切成當前輸入源後直接退出，不起 IMK
    if std::env::args().any(|argument| argument == "--register") {
        match app::input_source::register_main_bundle() {
            Ok(true) => println!("青简输入源已注册、启用并切成当前输入源"),
            Ok(false) => println!("青简输入源已注册并启用，请在输入法菜单里选择「青简」"),
            Err(error) => {
                eprintln!("青简输入源注册失败：{error}");
                std::process::exit(1);
            }
        }
        return;
    }
    let _log_guard = app::logging::init();
    // panic 的位置與 backtrace 記進日誌（攔截在 imk::catch_panic，這裡只記不碰狀態）；
    // 後臺執行緒（雲聯想）的 panic 也經過這裡，否則只會打到沒人看的 stderr
    std::panic::set_hook(Box::new(|info| {
        let backtrace = std::backtrace::Backtrace::force_capture();
        tracing::error!(%info, %backtrace, "panic");
    }));
    let info = app::BundleInfo::from_main_bundle();
    tracing::info!(connection = %info.connection_name, bundle = %info.identifier, "启动");

    // define_class! 的類在首次呼叫 class() 時才註冊到 ObjC 執行時，而 IMKServer 初始化時就會按
    // Info.plist 裡的類名查詢；找不到會靜默退回基類 IMKInputController，表現為按鍵全部透傳。
    // 所以必須先註冊類，再建 server。
    let controller_class = imk::QingjianInputController::class();
    tracing::info!(class = %controller_class.name().to_string_lossy(), "控制器类已注册");

    let mtm = MainThreadMarker::new().expect("输入法入口必须在主线程");
    if let Err(error) = host::init(mtm, &info) {
        tracing::error!(%error, "Engine 初始化失败");
        std::process::exit(1);
    }

    // IMKServer 需要在主執行緒建立，並在整個程序生命週期內活著
    let server = unsafe {
        IMKServer::initWithName_bundleIdentifier(
            IMKServer::alloc(),
            Some(&NSString::from_str(&info.connection_name)),
            Some(&NSString::from_str(&info.identifier)),
        )
    };
    let Some(_server) = server else {
        tracing::error!(
            "IMKServer 创建失败：检查 Info.plist 的 InputMethodConnectionName 与沙盒设置"
        );
        std::process::exit(1);
    };
    NSApplication::sharedApplication(mtm).run();
}
