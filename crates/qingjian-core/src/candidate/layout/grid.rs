//! 橫排展開成矩陣時的視口與高亮移動：一行就是一頁候選，固定顯示幾行，高亮移出視口時按行滾動。
//! 與分頁一樣是展示規則，各平臺殼共用，所以放在 Core；殼只轉發方向鍵。

use std::ops::Range;

use super::CandidateLayout;

/// 矩陣視口顯示幾行。
pub const GRID_ROWS: usize = 6;

/// 一格最多按幾個字寬留：四字詞能完整顯示，再長的候選在格子裡截斷。
pub const MAX_CELL_EMS: f32 = 4.0;

/// 估寬時非寬字元（拉丁字母、數字、半形標點）按幾個字寬算。
const NARROW_EMS: f32 = 0.62;

/// 雲端詞前面的雲朵圖示按幾個字寬算。
const CLOUD_EMS: f32 = 1.1;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Grid {
    /// 視口第一行的行號（頁號）。
    top: usize,
}

impl Grid {
    /// 從第 `row` 行展開：它當視口第一行；後面不夠一屏時視口往上補齊，視窗高度才不會因為在末幾頁展開而變矮。
    pub fn at(layout: &CandidateLayout, row: usize) -> Self {
        Self {
            top: row.min(layout.pages().saturating_sub(GRID_ROWS)),
        }
    }

    /// 各列要留幾個字寬：按**整份**候選估（不只是視口裡那幾行），同一輪查詢裡滾動、移動高亮時列寬不變，視窗不跳。
    /// 按字元數估而不實測——漢字一個字寬、拉丁字母不到一個——幾百條候選每幀實測太慢；上限 [`MAX_CELL_EMS`]。
    pub fn column_ems(layout: &CandidateLayout) -> Vec<f32> {
        let columns = layout.page_size();
        let mut widths = vec![0.0_f32; columns];
        for (i, cell) in layout.cells().iter().enumerate() {
            let Some(candidate) = cell.candidate() else {
                continue;
            };
            let mut ems: f32 = candidate
                .text
                .chars()
                .map(|c| if c.is_ascii() { NARROW_EMS } else { 1.0 })
                .sum();
            if matches!(cell, super::Cell::Cloud(_)) {
                ems += CLOUD_EMS;
            }
            let width = &mut widths[i % columns];
            *width = width.max(ems.min(MAX_CELL_EMS));
        }
        widths
    }

    pub fn top(&self) -> usize {
        self.top
    }

    /// 視口裡的行號，不超過排布的總行數。
    pub fn rows(&self, layout: &CandidateLayout) -> Range<usize> {
        let end = (self.top + GRID_ROWS).min(layout.pages().max(1));
        self.top.min(end)..end
    }

    /// 高亮豎著移 `delta` 行、列不變；目標格沒有候選（行尾不滿、固定位置留的空位）就取那一行裡離它最近的候選，
    /// 整行都沒有就順著方向再找下一行。返回新的高亮下標，動不了返回 `None`。視口跟著滾。
    pub fn move_rows(
        &mut self,
        layout: &CandidateLayout,
        highlighted: usize,
        delta: isize,
    ) -> Option<usize> {
        let columns = layout.page_size();
        let rows = layout.pages();
        if rows == 0 || delta == 0 {
            return None;
        }
        let (row, column) = (highlighted / columns, highlighted % columns);
        let mut target = (row as isize + delta).clamp(0, rows as isize - 1) as usize;
        while target != row {
            if let Some(index) = nearest_in_row(layout, target, column) {
                self.reveal(index / columns);
                return Some(index);
            }
            let next = target as isize + delta.signum();
            if next < 0 || next >= rows as isize {
                break;
            }
            target = next as usize;
        }
        None
    }

    /// 高亮按閱讀順序移一格（越過行尾到下一行開頭），跳過空位。返回新的高亮下標，到頭了返回 `None`。
    pub fn move_cells(
        &mut self,
        layout: &CandidateLayout,
        highlighted: usize,
        delta: isize,
    ) -> Option<usize> {
        let step = delta.signum();
        if step == 0 {
            return None;
        }
        let mut index = highlighted as isize + step;
        while index >= 0 && (index as usize) < layout.len() {
            if layout.candidate(index as usize).is_some() {
                self.reveal(index as usize / layout.page_size());
                return Some(index as usize);
            }
            index += step;
        }
        None
    }

    /// 滾動視口讓第 `row` 行可見：往上露出就讓它當第一行，往下露出就讓它當最後一行。
    fn reveal(&mut self, row: usize) {
        if row < self.top {
            self.top = row;
        } else if row >= self.top + GRID_ROWS {
            self.top = row + 1 - GRID_ROWS;
        }
    }
}

/// 第 `row` 行裡離第 `column` 列最近的候選下標（同距離取左邊的）。
fn nearest_in_row(layout: &CandidateLayout, row: usize, column: usize) -> Option<usize> {
    let columns = layout.page_size();
    let start = row * columns;
    (0..columns)
        .filter(|&c| layout.candidate(start + c).is_some())
        .min_by_key(|&c| (c.abs_diff(column), c))
        .map(|c| start + c)
}

#[cfg(test)]
mod tests {
    use super::{GRID_ROWS, Grid, MAX_CELL_EMS};
    use crate::candidate::{Candidate, CandidateKind, CandidateLayout};

    fn layout(count: usize, page_size: usize) -> CandidateLayout {
        let candidates = (0..count)
            .map(|i| Candidate {
                text: format!("本{i}"),
                kind: CandidateKind::Chinese,
                syllables: vec!["a".into()],
                reading: None,
                translation: None,
                aux_code: None,
            })
            .collect();
        CandidateLayout::new(candidates, page_size, 0)
    }

    #[test]
    fn rows_keep_the_column_and_clamp_to_the_last_candidate_of_a_short_row() {
        let layout = layout(13, 5);
        let mut grid = Grid::at(&layout, 0);
        // 第 0 行第 3 列 → 第 1 行第 3 列
        assert_eq!(grid.move_rows(&layout, 3, 1), Some(8));
        // 最後一行只有 3 個：第 3 列沒有，落到最近的第 2 列
        assert_eq!(grid.move_rows(&layout, 8, 1), Some(12));
        // 到底了再往下不動；往上回到同一列
        assert_eq!(grid.move_rows(&layout, 12, 1), None);
        assert_eq!(grid.move_rows(&layout, 12, -1), Some(7));
        assert_eq!(grid.move_rows(&layout, 2, -1), None);
    }

    #[test]
    fn viewport_scrolls_by_one_row_when_the_highlight_leaves_it() {
        let layout = layout(100, 5);
        let mut grid = Grid::at(&layout, 0);
        let mut highlighted = 0;
        for _ in 0..GRID_ROWS - 1 {
            highlighted = grid.move_rows(&layout, highlighted, 1).unwrap();
        }
        assert_eq!((highlighted / 5, grid.top()), (5, 0));
        assert_eq!(grid.rows(&layout), 0..6);
        // 再往下一行：視口滾一行
        highlighted = grid.move_rows(&layout, highlighted, 1).unwrap();
        assert_eq!((highlighted / 5, grid.top()), (6, 1));
        // 往上回到視口頂再往上：視口跟著往上滾
        for _ in 0..6 {
            highlighted = grid.move_rows(&layout, highlighted, -1).unwrap();
        }
        assert_eq!((highlighted / 5, grid.top()), (0, 0));
        // 整屏翻（翻頁鍵）：一次 6 行，末尾夾住
        highlighted = grid
            .move_rows(&layout, highlighted, GRID_ROWS as isize)
            .unwrap();
        assert_eq!((highlighted / 5, grid.top()), (6, 1));
        assert_eq!(grid.move_rows(&layout, highlighted, 1000), Some(95));
        assert_eq!(grid.rows(&layout), 14..20);
    }

    #[test]
    fn expanding_near_the_end_fills_the_viewport_upwards() {
        let layout = layout(100, 5);
        assert_eq!(Grid::at(&layout, 3).rows(&layout), 3..9);
        assert_eq!(Grid::at(&layout, 19).rows(&layout), 14..20);
        // 總共不到一屏：從頭顯示
        let short = super::super::CandidateLayout::new(Vec::new(), 5, 0);
        assert_eq!(Grid::at(&short, 0).top(), 0);
    }

    #[test]
    fn column_widths_come_from_the_whole_list_and_are_capped() {
        let mut candidates: Vec<Candidate> = ["是", "时候", "abc", "一心一意", "是不是因为我们"]
            .iter()
            .map(|text| Candidate {
                text: (*text).into(),
                kind: CandidateKind::Chinese,
                syllables: vec!["a".into()],
                reading: None,
                translation: None,
                aux_code: None,
            })
            .collect();
        // 第二行第一列有個三字詞：哪怕不在視口裡，第一列也按它留寬
        candidates.push(Candidate {
            text: "事实上".into(),
            ..candidates[0].clone()
        });
        let layout = CandidateLayout::new(candidates, 5, 0);
        let ems = Grid::column_ems(&layout);
        assert_eq!(ems.len(), 5);
        assert_eq!(ems[0], 3.0);
        assert_eq!(ems[1], 2.0);
        assert!((ems[2] - 1.86).abs() < 1e-5);
        assert_eq!(ems[3], 4.0);
        assert_eq!(ems[4], MAX_CELL_EMS);
    }

    #[test]
    fn cells_move_in_reading_order_across_rows() {
        let layout = layout(7, 5);
        let mut grid = Grid::at(&layout, 0);
        assert_eq!(grid.move_cells(&layout, 4, 1), Some(5));
        assert_eq!(grid.move_cells(&layout, 5, -1), Some(4));
        assert_eq!(grid.move_cells(&layout, 6, 1), None);
        assert_eq!(grid.move_cells(&layout, 0, -1), None);
    }
}
