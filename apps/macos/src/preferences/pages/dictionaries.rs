//! 「詞庫」頁：匯入按鈕，加一個可滾動的列表，每本一行（勾選框開關 + 非隨包的「移除」）。

use std::cell::RefCell;

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSScrollView, NSTextField, NSView};
use objc2_foundation::{NSPoint, NSRect, NSSize};

use crate::host::DictionaryInfo;
use crate::preferences::controls::{
    GROUP_GAP, NOTE_HEIGHT, button, checkbox, note_full, set_checked, small_label,
};
use crate::preferences::layout::{Layout, PAGE_PADDING, PAGE_WIDTH, ROW_HEIGHT};
use crate::preferences::setting::Setting;
use crate::preferences::target::PreferencesTarget;

/// 列表裡一行的高度。
const ROW: f64 = ROW_HEIGHT + 6.0;

/// 列表區的最小高度（約放得下 8 本）；視窗更高時撐到頁底，裝不下的滾。
const LIST_HEIGHT: f64 = 10.0 * ROW;

pub struct DictionariesPage {
    /// 列表的文件檢視，行都加在它上面。
    list: Retained<NSView>,

    /// 裝列表的滾動檢視。
    scroll: Retained<NSScrollView>,

    /// 當前的行控制元件，重建時先移除。
    rows: RefCell<Vec<Retained<NSView>>>,

    /// 一本都沒有時的提示。
    empty: Retained<NSTextField>,

    /// 行控制元件的 target；建行時用。
    target: Retained<PreferencesTarget>,

    mtm: MainThreadMarker,
}

impl DictionariesPage {
    pub fn build(
        layout: &mut Layout,
        mtm: MainThreadMarker,
        target: &Retained<PreferencesTarget>,
    ) -> Self {
        let import = button(mtm, "导入词库…", Setting::ImportDictionary, target);
        layout.place(&import, PAGE_PADDING, 140.0, ROW_HEIGHT + 4.0);
        layout.next_row(ROW_HEIGHT + 4.0);
        note_full(
            layout,
            mtm,
            "接受青简 TSV（词、拼音、词频三列）、Rime 的 .dict.yaml 和 .qj 文件，导入后立即可用；勾选框控制开关，「移除」把文件挪到词库目录的 removed 里，不会真删。",
        );
        layout.space(GROUP_GAP);
        let list = NSView::initWithFrame(mtm.alloc(), NSRect::ZERO);
        // 列表放在滾動視圖裡：隨包 11 本加匯入的可能超過一屏，全部列出、可以滾，不再只顯示前幾本
        let scroll = NSScrollView::initWithFrame(mtm.alloc(), NSRect::ZERO);
        scroll.setHasVerticalScroller(true);
        scroll.setDrawsBackground(false);
        scroll.setDocumentView(Some(&list));
        layout.place_fill(&scroll, PAGE_PADDING, layout.inner_width(), LIST_HEIGHT);
        layout.next_row(LIST_HEIGHT);
        let empty = small_label(mtm, "还没有附加词库。随包的基础词库不在这里，它一直启用。");
        list.addSubview(&empty);
        empty.setFrame(NSRect::new(
            NSPoint::new(0.0, LIST_HEIGHT - NOTE_HEIGHT),
            NSSize::new(PAGE_WIDTH - 2.0 * PAGE_PADDING, NOTE_HEIGHT),
        ));
        Self {
            list,
            scroll,
            rows: RefCell::new(Vec::new()),
            empty,
            target: target.clone(),
            mtm,
        }
    }

    /// 按當前詞庫清單重建列表：每本一行，勾選框（名字 · 條數 · 許可證）+「移除」按鈕。
    pub fn rebuild(&self, dictionaries: &[DictionaryInfo]) {
        let mtm = self.mtm;
        for view in self.rows.borrow_mut().drain(..) {
            view.removeFromSuperview();
        }
        self.empty.setHidden(!dictionaries.is_empty());
        let width = PAGE_WIDTH - 2.0 * PAGE_PADDING;
        // 文件檢視按行數撐高（至少一屏），行從頂部往下排；捲軸要留出位置
        let visible_height = self.scroll.contentSize().height.max(LIST_HEIGHT);
        let document_height = (ROW * dictionaries.len() as f64).max(visible_height);
        let content_width = self.scroll.contentSize().width.min(width);
        self.list.setFrame(NSRect::new(
            NSPoint::ZERO,
            NSSize::new(content_width, document_height),
        ));
        self.empty.setFrame(NSRect::new(
            NSPoint::new(0.0, document_height - NOTE_HEIGHT),
            NSSize::new(content_width, NOTE_HEIGHT),
        ));
        let width = content_width;
        let mut rows = self.rows.borrow_mut();
        for (index, info) in dictionaries.iter().enumerate() {
            let y = document_height - ROW * (index as f64 + 1.0);
            let mut title = info.name.clone();
            if info.broken {
                title.push_str("（文件损坏）");
            } else {
                title.push_str(&format!(" · {} 条", info.entries));
                if info.builtin {
                    title.push_str(" · 随包");
                } else if !info.license.is_empty() {
                    title.push_str(&format!(" · {}", info.license));
                }
            }
            let toggle = checkbox(mtm, &title, Setting::DictionaryEnabled(index), &self.target);
            set_checked(&toggle, info.enabled);
            toggle.setEnabled(!info.broken);
            toggle.setFrame(NSRect::new(
                NSPoint::new(0.0, y),
                NSSize::new(width - 80.0, ROW),
            ));
            self.list.addSubview(&toggle);
            rows.push(Retained::into_super(Retained::into_super(toggle)));
            // 隨包詞庫只能開關，不能移除
            if !info.builtin {
                let remove = button(mtm, "移除", Setting::DictionaryRemove(index), &self.target);
                remove.setFrame(NSRect::new(
                    NSPoint::new(width - 72.0, y + 1.0),
                    NSSize::new(72.0, ROW - 2.0),
                ));
                self.list.addSubview(&remove);
                rows.push(Retained::into_super(Retained::into_super(remove)));
            }
        }
        // 非翻轉座標系的文件檢視預設停在底部，滾回頂部讓第一本在最上面
        let clip = self.scroll.contentView();
        clip.scrollToPoint(NSPoint::new(
            0.0,
            document_height - clip.bounds().size.height,
        ));
        self.scroll.reflectScrolledClipView(&clip);
    }
}
