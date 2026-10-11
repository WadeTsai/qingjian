//! 候選窗字型的選擇控制元件：外面是和其他設定項一樣的彈出按鈕，點開是帶搜尋框的字型列表（NSPopover），選中即生效。
//! 過濾按子串，輸入法上屏的字母一樣算數；每行用自己的字型顯示名字。

mod button;
mod source;
mod state;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{MainThreadMarker, sel};
use objc2_app_kit::{
    NSBorderType, NSPopover, NSPopoverBehavior, NSScrollView, NSSearchField, NSTableColumn,
    NSTableView, NSView, NSViewController,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

use self::button::FontPopUpButton;
use self::source::FontListSource;
use super::controls::caption;
use super::layout::{CONTROL_X, LABEL_WIDTH, Layout, PAGE_PADDING, ROW_HEIGHT};

/// 下拉框寬度。
const POPOVER_WIDTH: f64 = 280.0;

/// 列表高度：夠看十來行。
const LIST_HEIGHT: f64 = 240.0;

/// 下拉框內邊距。
const INSET: f64 = 6.0;

pub(super) struct FontPicker {
    /// 資料來源，持有搜尋框、列表、下拉框與過濾狀態。
    source: Retained<FontListSource>,
}

impl FontPicker {
    /// 放一行「標題 + 彈出按鈕」；搜尋框和列表在下拉框裡，點按鈕才出現。
    pub(super) fn build(
        layout: &mut Layout,
        mtm: MainThreadMarker,
        title: &str,
        families: Vec<String>,
    ) -> Self {
        let source = FontListSource::new(mtm, families);
        let button = FontPopUpButton::new(mtm, source.clone());
        let label = caption(mtm, title);
        layout.place(&label, PAGE_PADDING, LABEL_WIDTH, ROW_HEIGHT);
        layout.place(
            &button,
            CONTROL_X,
            layout.control_width().min(200.0),
            ROW_HEIGHT,
        );
        layout.next_row(ROW_HEIGHT);

        let inner_width = POPOVER_WIDTH - INSET * 2.0;
        let search = NSSearchField::initWithFrame(
            mtm.alloc(),
            NSRect::new(
                NSPoint::new(INSET, INSET + LIST_HEIGHT + INSET),
                NSSize::new(inner_width, ROW_HEIGHT),
            ),
        );
        search.setPlaceholderString(Some(&NSString::from_str("输入几个字母筛选")));
        search.setSendsWholeSearchString(false);
        search.setSendsSearchStringImmediately(true);
        // SAFETY: 選擇器與 FontListSource 上定義的 `filterChanged:` 一致，簽名 (id) -> void
        unsafe {
            search.setTarget(Some(&*source));
            search.setAction(Some(sel!(filterChanged:)));
        }

        let table = NSTableView::initWithFrame(mtm.alloc(), NSRect::ZERO);
        table.setRowHeight(source::ROW);
        table.setAllowsMultipleSelection(false);
        table.setAllowsEmptySelection(false);
        table.setHeaderView(None);
        let column = NSTableColumn::initWithIdentifier(mtm.alloc(), &NSString::from_str("family"));
        column.setWidth(inner_width - 20.0);
        table.addTableColumn(&column);
        // SAFETY: 資料來源隨頁面活著，比表格活得久
        unsafe {
            table.setDataSource(Some(ProtocolObject::from_ref(&*source)));
            table.setDelegate(Some(ProtocolObject::from_ref(&*source)));
        }
        let list = NSScrollView::initWithFrame(
            mtm.alloc(),
            NSRect::new(
                NSPoint::new(INSET, INSET),
                NSSize::new(inner_width, LIST_HEIGHT),
            ),
        );
        list.setHasVerticalScroller(true);
        list.setBorderType(NSBorderType::BezelBorder);
        list.setDocumentView(Some(&table));

        let content_size = NSSize::new(POPOVER_WIDTH, INSET * 3.0 + ROW_HEIGHT + LIST_HEIGHT);
        let content = NSView::initWithFrame(mtm.alloc(), NSRect::new(NSPoint::ZERO, content_size));
        content.addSubview(&list);
        content.addSubview(&search);
        let controller = NSViewController::init(mtm.alloc());
        controller.setView(&content);
        let popover = NSPopover::init(mtm.alloc());
        popover.setContentViewController(Some(&controller));
        popover.setContentSize(content_size);
        // 點到下拉框外面就收起
        popover.setBehavior(NSPopoverBehavior::Transient);
        source.attach(Retained::into_super(button), search, table, popover);
        Self { source }
    }

    /// 照配置顯示，不查有沒有裝；沒裝由渲染器回退系統字型。
    pub(super) fn sync(&self, font: &str) {
        self.source.set_current(font);
    }
}
