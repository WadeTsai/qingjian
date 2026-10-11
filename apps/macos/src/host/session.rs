//! 當前輸入會話的 UI 狀態：候選排布、高亮、頁碼、preedit。
//!
//! 放在 Host 裡而不是控制器的 ivars 裡，是因為聯想結果由定時器送達，那時手上沒有控制器；
//! 反正 Engine 的緩衝區也是全程序一份，會話狀態跟著它走。
//! 候選的分頁與雲端詞的位置由 Core 的 [`CandidateLayout`] 定，這裡只管高亮與頁碼。

use qingjian_core::{Candidate, CandidateLayout, Cell, GRID_ROWS, Grid};

use crate::candidates::Preedit;

#[derive(Debug, Default)]
pub struct Session {
    /// 上一次查詢的候選排布（本地候選 + 雲端詞）。
    pub layout: CandidateLayout,

    /// 高亮的格子下標（在整個排布里的絕對位置）。
    pub highlighted: usize,

    /// 當前頁。
    pub page: usize,

    /// 這輪查詢裡使用者用方向鍵 / 翻頁鍵動過高亮。動過就不再拿重排結果換掉候選。
    pub navigated: bool,

    /// 候選視窗頂部顯示的拼音行（分段 + 游標）。
    pub preedit: Option<Preedit>,

    /// 橫排展開成矩陣時的視口；`None` 是單行。新一輪查詢收回單行。
    pub grid: Option<Grid>,
}

impl Session {
    /// 新一輪查詢：候選換掉，選中第一個真實候選。
    pub fn reset(
        &mut self,
        preedit: Option<Preedit>,
        candidates: Vec<Candidate>,
        page_size: usize,
        slots: usize,
    ) {
        self.preedit = preedit;
        self.layout = CandidateLayout::new(candidates, page_size, slots);
        self.highlighted = (0..self.layout.len())
            .find(|&i| self.layout.candidate(i).is_some())
            .unwrap_or(0);
        self.page = self.highlighted / self.layout.page_size();
        self.navigated = false;
        self.grid = None;
    }

    /// 矩陣裡豎著移 `delta` 行；還是單行就先從當前頁展開（展開本身也算變化）。返回要不要重畫。
    pub fn move_rows(&mut self, delta: isize) -> bool {
        let expanding = self.grid.is_none();
        let mut grid = self
            .grid
            .unwrap_or_else(|| Grid::at(&self.layout, self.page));
        let moved = grid.move_rows(&self.layout, self.highlighted, delta);
        self.grid = Some(grid);
        self.land(moved) || expanding
    }

    /// 矩陣裡整屏翻（翻頁鍵）：一次 [`GRID_ROWS`] 行。
    pub fn move_screens(&mut self, delta: isize) -> bool {
        self.move_rows(delta * GRID_ROWS as isize)
    }

    /// 矩陣裡按閱讀順序移一格；沒展開時不動。
    pub fn move_cells(&mut self, delta: isize) -> bool {
        let Some(mut grid) = self.grid else {
            return false;
        };
        let moved = grid.move_cells(&self.layout, self.highlighted, delta);
        self.grid = Some(grid);
        self.land(moved)
    }

    /// 收回單行，高亮留在原處。返回原來是不是展開著。
    pub fn collapse(&mut self) -> bool {
        self.grid.take().is_some()
    }

    /// 矩陣視口裡的格子（按行優先排開）與每行幾格；沒展開返回 `None`。
    pub fn grid_cells(&self) -> Option<(Vec<Cell<'_>>, usize)> {
        let grid = self.grid?;
        let columns = self.layout.page_size();
        let mut cells = Vec::new();
        for row in grid.rows(&self.layout) {
            let mut page = self.layout.page(row);
            page.resize(columns, Cell::Empty);
            cells.extend(page);
        }
        Some((cells, columns))
    }

    /// 高亮落到 `index`（`None` 是沒動），頁號跟著走。
    fn land(&mut self, index: Option<usize>) -> bool {
        let Some(index) = index else {
            return false;
        };
        self.highlighted = index;
        self.page = index / self.layout.page_size();
        self.navigated = true;
        true
    }

    /// 第 `index` 格的候選。
    pub fn candidate(&self, index: usize) -> Option<Candidate> {
        self.layout.candidate(index).cloned()
    }

    /// 當前頁第 `offset` 格在整個排布里的下標；越界返回 `None`。
    pub fn index_on_page(&self, offset: usize) -> Option<usize> {
        let index = self.page * self.layout.page_size() + offset;
        (offset < self.layout.page_size() && index < self.layout.len()).then_some(index)
    }

    /// 候選窗口裡第 `offset` 格在整個排布里的下標：矩陣展開時按視口行優先數（視口是從 `top` 起連續的幾頁），
    /// 沒展開時同 [`Self::index_on_page`]。越界返回 `None`。點選上屏用。
    pub fn index_in_view(&self, offset: usize) -> Option<usize> {
        let Some(grid) = self.grid else {
            return self.index_on_page(offset);
        };
        let index = grid.top() * self.layout.page_size() + offset;
        (index < self.layout.len()).then_some(index)
    }

    /// 當前頁的格子。
    pub fn page_cells(&self) -> Vec<Cell<'_>> {
        self.layout.page(self.page)
    }

    /// 高亮上下移動，越過頁邊自動翻頁。返回是否有變化。
    pub fn move_highlight(&mut self, delta: isize) -> bool {
        let len = self.layout.len();
        if len == 0 {
            return false;
        }
        let current = self.highlighted as isize;
        let mut next = (current + delta).clamp(0, len as isize - 1) as usize;
        while self.layout.candidate(next).is_none() {
            let candidate = next as isize + delta.signum();
            if candidate < 0 || candidate >= len as isize || delta == 0 {
                return false;
            }
            next = candidate as usize;
        }
        if next == self.highlighted {
            return false;
        }
        self.highlighted = next;
        self.page = next / self.layout.page_size();
        self.navigated = true;
        true
    }

    /// 翻頁並選中新頁第一個真實候選，跳過沒有候選的頁。
    pub fn turn_page(&mut self, delta: isize) -> bool {
        let pages = self.layout.pages().max(1);
        let current = self.page as isize;
        let mut next = (current + delta).clamp(0, pages as isize - 1) as usize;
        loop {
            if next == self.page {
                return false;
            }
            let start = next * self.layout.page_size();
            if let Some(index) = (start..(start + self.layout.page_size()).min(self.layout.len()))
                .find(|&i| self.layout.candidate(i).is_some())
            {
                self.page = next;
                self.highlighted = index;
                break;
            }
            let following = next as isize + delta.signum();
            if following < 0 || following >= pages as isize || delta == 0 {
                return false;
            }
            next = following as usize;
        }
        self.navigated = true;
        true
    }

    pub fn pages(&self) -> usize {
        self.layout.pages()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qingjian_core::CandidateKind;

    fn candidates(count: usize) -> Vec<Candidate> {
        (0..count)
            .map(|i| Candidate {
                text: format!("本{i}"),
                kind: CandidateKind::Chinese,
                syllables: vec!["a".into()],
                reading: None,
                translation: None,
                aux_code: None,
            })
            .collect()
    }

    #[test]
    fn vertical_keys_expand_the_row_into_a_scrolling_grid() {
        let mut session = Session::default();
        session.reset(None, candidates(100), 5, 0);
        assert!(session.grid_cells().is_none());
        // 左右鍵在單行時不歸候選管
        assert!(!session.move_cells(1));
        // 第一下往下：展開並換到第二行同一列，數字鍵跟著選第二行
        assert!(session.move_rows(1));
        assert_eq!((session.highlighted, session.page), (5, 1));
        assert_eq!(session.index_on_page(2), Some(7));
        let (cells, columns) = session.grid_cells().unwrap();
        assert_eq!((cells.len(), columns), (30, 5));
        // 展開後左右鍵按閱讀順序移動，越過行尾到下一行
        assert!(session.move_cells(-1));
        assert_eq!((session.highlighted, session.page), (4, 0));
        // 一直往下：視口按行滾動
        for _ in 0..7 {
            session.move_rows(1);
        }
        assert_eq!(session.page, 7);
        assert_eq!(session.grid.unwrap().top(), 2);
        // 翻頁鍵一次一屏；Esc 收回單行，高亮不動
        assert!(session.move_screens(1));
        assert_eq!(session.page, 13);
        assert!(session.collapse());
        assert!(!session.collapse());
        assert_eq!(session.page, 13);
        // 新一輪查詢回到單行
        session.move_rows(1);
        session.reset(None, candidates(3), 5, 0);
        assert!(session.grid.is_none());
        // 只有一行時往上：展開但不動，仍要重畫
        assert!(session.move_rows(-1));
        let (cells, _) = session.grid_cells().unwrap();
        assert_eq!(cells.len(), 5);
    }

    #[test]
    fn clicks_in_an_expanded_grid_map_to_viewport_cells() {
        let mut session = Session::default();
        session.reset(None, candidates(100), 5, 0);
        assert_eq!(session.index_in_view(3), Some(3));
        for _ in 0..7 {
            session.move_rows(1);
        }
        let top = session.grid.unwrap().top();
        assert!(top > 0);
        // 視口第二行第三格：不是當前頁的第 8 格
        assert_eq!(session.index_in_view(7), Some(top * 5 + 7));
        assert_ne!(session.index_in_view(7), session.index_on_page(7));
    }

    #[test]
    fn digits_map_to_cells_on_the_current_page() {
        let mut session = Session::default();
        session.reset(None, candidates(3), 9, 2);
        assert_eq!(session.index_on_page(2), Some(2));
        assert_eq!(session.index_on_page(3), None);
        assert!(session.move_highlight(1));
        assert!(session.move_highlight(5));
        assert_eq!(session.highlighted, 2);
        assert!(!session.move_highlight(1));
    }

    #[test]
    fn paging_follows_the_layout() {
        let mut session = Session::default();
        session.reset(None, candidates(12), 9, 2);
        assert_eq!(session.pages(), 2);
        assert!(session.turn_page(1));
        assert_eq!(session.highlighted, 9);
        assert_eq!(session.index_on_page(0), Some(9));
        assert!(!session.turn_page(1));
    }

    #[test]
    fn navigation_is_remembered_until_the_next_query() {
        let mut session = Session::default();
        session.reset(None, candidates(12), 9, 2);
        assert!(!session.navigated);
        // 頂到邊界沒動算沒導航
        assert!(!session.move_highlight(-1));
        assert!(!session.navigated);
        assert!(session.move_highlight(1));
        assert!(session.navigated);
        session.reset(None, candidates(3), 9, 2);
        assert!(!session.navigated);
        // 只有一頁時翻頁沒動，也不算導航
        assert!(!session.turn_page(1));
        assert!(!session.navigated);
        session.reset(None, candidates(12), 9, 2);
        assert!(session.turn_page(1));
        assert!(session.navigated);
    }

    #[test]
    fn sparse_custom_positions_keep_highlight_on_real_candidates() {
        let mut words = candidates(1);
        words[0].kind = CandidateKind::Custom(9);
        let mut session = Session::default();
        session.reset(None, words.clone(), 5, 2);
        assert_eq!((session.page, session.highlighted), (1, 8));
        assert!(!session.turn_page(-1));
        words.extend(candidates(1));
        session.reset(None, words, 5, 2);
        assert_eq!((session.page, session.highlighted), (0, 0));
        assert!(session.turn_page(1));
        assert_eq!((session.page, session.highlighted), (1, 8));
        assert!(session.move_highlight(-1));
        assert_eq!((session.page, session.highlighted), (0, 0));
    }
}
