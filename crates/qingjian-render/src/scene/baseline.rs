//! 表格一行裡的各格按第一行文字的基線對齊。
//!
//! 主題用上邊距按自己的字號把序號、候選、譯文手調到同一條基線；設定裡候選與譯文按不同比例改了字號後，
//! 這裡量出差多少，補到偏高那幾格的上邊距上。不用 taffy 自帶的基線對齊：巢狀的框上報的基線是 0，且它把上邊距也算進基線。

use std::collections::HashMap;

use taffy::{LengthPercentageAuto, NodeId, Position};

use super::{Scene, SceneNode, Visual};
use crate::error::RenderError;
use crate::text::TextSize;

impl Scene {
    /// 登記表格的一行，`cells` 不含橫跨整行的格子。
    pub(crate) fn add_table_row(&mut self, cells: Vec<NodeId>) {
        if cells.len() > 1 {
            self.table_rows.push(cells);
        }
    }

    /// 把每行基線偏高的格子往下挪，返回挪沒挪（挪了要重新佈局）。差不到一畫素的不動，按主題自己的字號顯示時一格都不挪。
    pub(super) fn align_baselines(
        &mut self,
        measured: &HashMap<NodeId, TextSize>,
    ) -> Result<bool, RenderError> {
        let mut shifts = Vec::new();
        for row in &self.table_rows {
            let mut baselines = Vec::with_capacity(row.len());
            for &cell in row {
                if let Some(inner) = self.first_baseline(cell, measured)? {
                    baselines.push((cell, self.tree.layout(cell)?.location.y + inner));
                }
            }
            let lowest = baselines
                .iter()
                .map(|&(_, baseline)| baseline)
                .fold(f32::MIN, f32::max);
            for (cell, baseline) in baselines {
                let shift = (lowest - baseline).round();
                if shift >= 1.0 {
                    shifts.push((cell, shift));
                }
            }
        }
        for &(cell, shift) in &shifts {
            let top = self.tree.layout(cell)?.margin.top;
            let mut style = self.tree.style(cell)?.clone();
            style.margin.top = LengthPercentageAuto::length(top + shift);
            self.tree.set_style(cell, style)?;
        }
        Ok(!shifts.is_empty())
    }

    /// 節點裡第一段文字的基線離節點頂邊多遠；絕對定位的子節點（高亮條這類疊層）不算。
    fn first_baseline(
        &self,
        node: NodeId,
        measured: &HashMap<NodeId, TextSize>,
    ) -> Result<Option<f32>, RenderError> {
        if let Some(SceneNode {
            visual: Visual::Text { .. },
            ..
        }) = self.tree.get_node_context(node)
        {
            return Ok(measured.get(&node).map(|size| size.baseline));
        }
        for child in self.tree.children(node)? {
            if self.tree.style(child)?.position == Position::Absolute {
                continue;
            }
            if let Some(inner) = self.first_baseline(child, measured)? {
                return Ok(Some(self.tree.layout(child)?.location.y + inner));
            }
        }
        Ok(None)
    }
}
