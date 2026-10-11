use objc2_foundation::{NSBundle, NSString};

/// 從主 bundle 的 Info.plist 讀出的輸入法身份資訊。
#[derive(Debug, Clone)]
pub struct BundleInfo {
    /// `InputMethodConnectionName`，IMK 用它註冊 NSConnection。
    pub connection_name: String,

    /// `CFBundleIdentifier`。
    pub identifier: String,

    /// `CFBundleShortVersionString`，選單裡顯示。
    pub version: String,

    /// 構建標識：打包指令碼通過環境變數 `QINGJIAN_BUILD` 塞進來的 git 短雜湊與日期；直接 `cargo build` 的是「本地構建」。
    pub build: String,
}

impl BundleInfo {
    /// 讀不到時回退到編譯期常量，方便在 `.app` 之外直接跑二進位制看日誌。
    pub fn from_main_bundle() -> Self {
        let bundle = NSBundle::mainBundle();
        let identifier = bundle
            .bundleIdentifier()
            .map(|s| s.to_string())
            .unwrap_or_else(|| DEFAULT_IDENTIFIER.to_owned());
        let connection_name = bundle
            .objectForInfoDictionaryKey(&NSString::from_str("InputMethodConnectionName"))
            .and_then(|value| value.downcast::<NSString>().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("{identifier}_Connection"));
        let version = bundle
            .objectForInfoDictionaryKey(&NSString::from_str("CFBundleShortVersionString"))
            .and_then(|value| value.downcast::<NSString>().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_owned());
        Self {
            connection_name,
            identifier,
            version,
            build: option_env!("QINGJIAN_BUILD")
                .unwrap_or("本地构建")
                .to_owned(),
        }
    }
}

/// 與 `Info.plist` 裡的 `CFBundleIdentifier` 保持一致。
pub(crate) const DEFAULT_IDENTIFIER: &str = "app.qingjian.inputmethod";
