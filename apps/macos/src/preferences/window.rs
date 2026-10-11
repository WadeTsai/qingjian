//! 偏好設定視窗本體：把各頁（`pages/`）裝進標籤檢視，底部一行狀態；重新整理時逐頁同步。

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSClipView, NSColor, NSScreen, NSScrollView, NSTabView, NSTabViewItem, NSTextField, NSView,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use qingjian_core::{Language, UsageSummary, VocabularySummary};
use qingjian_platform::Config;

use super::controls::{language_label, small_label};
use super::layout::{Layout, PAGE_PADDING, PAGE_WIDTH};
use super::pages::{
    AboutPage, AdvancedPage, CandidatesPage, CloudPage, DictionariesPage, FuzzyPage, GeneralPage,
    PhrasesPage, ShortcutsPage, UpdateStatus, UsagePage, build_about,
};
use super::panel::PreferencesPanel;
use super::target::PreferencesTarget;
use crate::host::DictionaryInfo;

/// 每頁頂部留白、頁面最低高度（矮頁也撐到這個高度，切頁時視窗不跳）。
const PAGE_TOP: f64 = 18.0;
const MIN_PAGE_HEIGHT: f64 = 250.0;

/// 標籤檢視四周留白、底部狀態行高度。
const TAB_MARGIN: f64 = 14.0;
const STATUS_HEIGHT: f64 = 18.0;

/// 視窗比螢幕可用高度至少矮這麼多（標題欄 + 上下留一點邊）；頁面比視窗高時自己滾。
const SCREEN_MARGIN: f64 = 80.0;

/// 設定視窗與需要按配置重新整理的各頁。
pub struct PreferencesWindow {
    /// 視窗。
    panel: Retained<PreferencesPanel>,

    /// 「通用」頁。
    general: GeneralPage,

    /// 「候選視窗」頁。
    candidates: CandidatesPage,

    /// 「快捷鍵」頁。
    shortcuts: ShortcutsPage,

    /// 自定義短語編輯。
    phrases: PhrasesPage,

    /// 「模糊音」頁。
    fuzzy: FuzzyPage,

    /// 「詞庫」頁。
    dictionaries: DictionariesPage,

    /// 「雲服務」頁。
    cloud: CloudPage,

    /// 「進階」頁。
    advanced: AdvancedPage,

    /// 「統計」頁的數字。
    usage: UsagePage,

    /// 「關於」頁的檢查更新控制元件。
    about: AboutPage,

    /// 底部狀態行：配置檔案解析失敗時顯示原因，也給臨時提示用。
    status: Retained<NSTextField>,

    /// 所有控制元件的 target，要和視窗活得一樣久。
    _target: Retained<PreferencesTarget>,
}

/// 一頁：標題、佈局器、承載檢視。
type Page = (&'static str, Layout, Retained<NSView>);

impl PreferencesWindow {
    /// `languages` 是打進包裡的釋義表語言，`version` / `build` 顯示在「關於」頁。
    pub fn new(mtm: MainThreadMarker, languages: &[Language], version: &str, build: &str) -> Self {
        let target = PreferencesTarget::new(mtm);
        let new_layout = || Layout::new(PAGE_WIDTH, PAGE_TOP);
        let page = |title: &'static str, layout: Layout| -> Page {
            (
                title,
                layout,
                NSView::initWithFrame(mtm.alloc(), NSRect::ZERO),
            )
        };
        let mut pages: Vec<Page> = Vec::new();

        let mut layout = new_layout();
        let general = GeneralPage::build(&mut layout, mtm, &target, languages);
        pages.push(page("通用", layout));

        let mut layout = new_layout();
        let candidates = CandidatesPage::build(&mut layout, mtm, &target);
        pages.push(page("候选窗口", layout));

        let mut layout = new_layout();
        let shortcuts = ShortcutsPage::build(&mut layout, mtm, &target);
        pages.push(page("快捷键", layout));

        let mut layout = new_layout();
        let phrases = PhrasesPage::build(&mut layout, mtm, &target);
        pages.push(page("自定义短语", layout));

        let mut layout = new_layout();
        let fuzzy = FuzzyPage::build(&mut layout, mtm, &target);
        pages.push(page("模糊音", layout));

        let mut layout = new_layout();
        let dictionaries = DictionariesPage::build(&mut layout, mtm, &target);
        pages.push(page("词库", layout));

        let mut layout = new_layout();
        let cloud = CloudPage::build(&mut layout, mtm, &target);
        pages.push(page("云服务", layout));

        let mut layout = new_layout();
        let advanced = AdvancedPage::build(&mut layout, mtm, &target);
        pages.push(page("高级", layout));

        let mut layout = new_layout();
        let usage = UsagePage::build(&mut layout, mtm);
        pages.push(page("统计", layout));

        let mut layout = new_layout();
        let about = build_about(&mut layout, mtm, &target, version, build);
        pages.push(page("关于", layout));

        // 標籤檢視：先用臨時尺寸量出邊框與標籤欄佔多少，再按最高的一頁定最終尺寸
        let tallest = pages
            .iter()
            .map(|(_, layout, _)| layout.height() + PAGE_TOP)
            .fold(MIN_PAGE_HEIGHT, f64::max);
        // 設定項多了以後最高的一頁會超出小螢幕，視窗底部（狀態行）掉到程式塢後面：視窗封頂，超高的頁放進滾動檢視
        let page_height = tallest.min(max_page_height(mtm)).max(MIN_PAGE_HEIGHT);
        let probe = NSRect::new(NSPoint::ZERO, NSSize::new(PAGE_WIDTH, page_height));
        let tabs = NSTabView::initWithFrame(mtm.alloc(), probe);
        let inner = tabs.contentRect();
        let chrome_width = PAGE_WIDTH - inner.size.width;
        let chrome_height = page_height - inner.size.height;
        let tabs_size = NSSize::new(PAGE_WIDTH + chrome_width, page_height + chrome_height);
        let content_size = NSSize::new(
            tabs_size.width + 2.0 * TAB_MARGIN,
            tabs_size.height + 2.0 * TAB_MARGIN + STATUS_HEIGHT,
        );
        tabs.setFrame(NSRect::new(
            NSPoint::new(TAB_MARGIN, TAB_MARGIN + STATUS_HEIGHT),
            tabs_size,
        ));
        for (title, layout, view) in pages {
            let own_height = (layout.height() + PAGE_TOP).max(page_height);
            view.setFrame(NSRect::new(
                NSPoint::ZERO,
                NSSize::new(PAGE_WIDTH, own_height),
            ));
            layout.finish(&view, own_height);
            // SAFETY: identifier 允許為空；條目隨 NSTabView 活著
            let item = unsafe { NSTabViewItem::initWithIdentifier(mtm.alloc(), None) };
            item.setLabel(&NSString::from_str(title));
            if own_height > page_height {
                item.setView(Some(&scrolling(mtm, &view, page_height, own_height)));
            } else {
                item.setView(Some(&view));
            }
            tabs.addTabViewItem(&item);
        }
        let content = NSView::initWithFrame(mtm.alloc(), NSRect::new(NSPoint::ZERO, content_size));
        content.addSubview(&tabs);
        let status = small_label(mtm, "");
        status.setTextColor(Some(&NSColor::systemRedColor()));
        status.setFrame(NSRect::new(
            NSPoint::new(TAB_MARGIN + PAGE_PADDING, TAB_MARGIN / 2.0),
            NSSize::new(
                content_size.width - 2.0 * (TAB_MARGIN + PAGE_PADDING),
                STATUS_HEIGHT,
            ),
        ));
        content.addSubview(&status);
        let panel = PreferencesPanel::new(mtm, NSRect::new(NSPoint::ZERO, content_size));
        panel.setTitle(&NSString::from_str("青简偏好设置"));
        panel.setContentView(Some(&content));
        panel.center();

        Self {
            panel,
            general,
            candidates,
            shortcuts,
            phrases,
            fuzzy,
            dictionaries,
            cloud,
            advanced,
            usage,
            about,
            status,
            _target: target,
        }
    }

    pub fn select_phrase(&self, config: &Config, index: usize) {
        self.phrases.load(config, index);
    }
    pub fn selected_phrase(&self) -> Option<usize> {
        self.phrases.selected_row()
    }
    pub fn edit_phrase(&self, config: &Config, index: Option<usize>) {
        self.phrases.edit(config, index);
    }
    pub fn close_phrase_editor(&self) {
        self.phrases.close_editor();
    }
    pub fn set_phrase_error(&self, error: &str) {
        self.phrases.set_error(error);
    }
    pub fn phrase_draft(
        &self,
        config: &Config,
    ) -> Result<(Option<usize>, qingjian_core::CustomPhrase), String> {
        Ok((self.phrases.selected(config)?, self.phrases.draft()))
    }

    /// 開啟（或帶到最前）。
    pub fn show(&self) {
        self.panel.present();
    }

    /// 按配置重新整理所有控制元件。`key_present` 是金鑰已經有了（環境或配置裡）；金鑰框永遠不回顯值。
    pub fn sync(
        &self,
        config: &Config,
        key_present: bool,
        error: Option<&str>,
        dictionaries: &[DictionaryInfo],
        update: &UpdateStatus,
    ) {
        self.dictionaries.rebuild(dictionaries);
        self.about.sync(config, update);
        self.general.sync(config);
        self.candidates.sync(config);
        self.shortcuts.sync(config);
        self.phrases.sync(config);
        self.fuzzy.sync(config);
        self.cloud.sync(
            config,
            key_present,
            crate::app::paths::p2c_model_path().is_some()
                || crate::app::paths::model_path().is_some(),
        );
        self.advanced.sync(config);
        let status = error
            .map(|e| format!("配置文件有错误，已沿用上一份：{e}"))
            .unwrap_or_default();
        self.status.setTextColor(Some(&NSColor::systemRedColor()));
        self.status.setStringValue(&NSString::from_str(&status));
    }

    /// 檢查更新的狀態變了（查完了、查到新版），只刷「關於」頁。
    pub fn sync_update(&self, config: &Config, update: &UpdateStatus) {
        self.about.sync(config, update);
    }

    /// 重新整理「統計」頁。開啟視窗時調（數字隨時在變，不跟配置一起同步）。
    pub fn sync_usage(
        &self,
        summary: &UsageSummary,
        vocabulary: &VocabularySummary,
        language: Option<Language>,
    ) {
        self.usage.show(
            summary,
            vocabulary,
            language.map_or("学习语言已关", language_label),
        );
    }

    /// 底部狀態行臨時顯示一句提示（不是錯誤，灰字）；下次 `sync` 會被配置狀態覆蓋。
    pub fn set_status(&self, text: &str) {
        self.status
            .setTextColor(Some(&NSColor::secondaryLabelColor()));
        self.status.setStringValue(&NSString::from_str(text));
    }
}

/// 一頁最高能多高：主屏可用高度減去標題欄、標籤欄、狀態行與留白。取不到螢幕就不封頂。
fn max_page_height(mtm: MainThreadMarker) -> f64 {
    NSScreen::mainScreen(mtm).map_or(f64::MAX, |screen| {
        screen.visibleFrame().size.height - SCREEN_MARGIN - 2.0 * TAB_MARGIN - STATUS_HEIGHT
    })
}

/// 把比視窗高的一頁放進滾動檢視，開始時停在頁頂。
fn scrolling(
    mtm: MainThreadMarker,
    page: &NSView,
    visible_height: f64,
    page_height: f64,
) -> Retained<NSScrollView> {
    let scroll = NSScrollView::initWithFrame(
        mtm.alloc(),
        NSRect::new(NSPoint::ZERO, NSSize::new(PAGE_WIDTH, visible_height)),
    );
    scroll.setHasVerticalScroller(true);
    scroll.setAutohidesScrollers(true);
    scroll.setDrawsBackground(false);
    scroll.setDocumentView(Some(page));
    // 頁面檢視沒有翻轉座標，頁頂在 y 最大處
    let clip: Retained<NSClipView> = scroll.contentView();
    clip.scrollToPoint(NSPoint::new(0.0, page_height - visible_height));
    scroll.reflectScrolledClipView(&clip);
    scroll
}
