//! 載入時檢查一遍引用（顏色變數、文字樣式、元件），有問題記警告；渲染時引用不到的退回預設值，不讓整個主題失敗。

use super::file::node::{BorderSpec, FillSpec, NodeKind, NodeSpec};
use super::file::{ColorRef, ColorSpec, FontRef, ThemeFile};

/// 列出主題裡引用不到的名字。
pub(super) fn problems(file: &ThemeFile) -> Vec<String> {
    let mut found = Vec::new();
    let mut check = Checker {
        file,
        found: &mut found,
    };
    for (name, component) in &file.components {
        check.node(component, &format!("components.{name}"));
    }
    check.node(&file.windows.vertical, "windows.vertical");
    check.node(&file.windows.horizontal, "windows.horizontal");
    let status = &file.status;
    if let Some(root) = &status.root {
        check.node(root, "status.root");
    }
    check.font(&status.font, "status");
    for color in [
        &status.background,
        &status.separator,
        &status.normal,
        &status.emphasized,
        &status.gear,
    ] {
        check.color(color, "status");
    }
    found
}

struct Checker<'a> {
    file: &'a ThemeFile,

    found: &'a mut Vec<String>,
}

impl Checker<'_> {
    fn node(&mut self, node: &NodeSpec, path: &str) {
        match &node.kind {
            NodeKind::Frame {
                fill,
                border,
                children,
                ..
            } => {
                for color in fill.iter().flat_map(FillSpec::colors) {
                    self.spec(color, path);
                }
                if let Some(border) = border {
                    self.spec(&border.color, path);
                }
                for (i, child) in children.iter().enumerate() {
                    self.node(child, &format!("{path}.children[{i}]"));
                }
            }
            NodeKind::Text {
                font,
                color,
                stroke,
                ..
            } => {
                self.font(font, path);
                self.spec(color, path);
                self.stroke(stroke.as_ref(), path);
            }
            NodeKind::Icon { color, .. } => self.spec(color, path),
            NodeKind::Preedit {
                font,
                typed,
                rest,
                struck,
                caret,
                stroke,
            } => {
                self.font(font, path);
                self.stroke(stroke.as_ref(), path);
                for color in [typed, rest, struck, &caret.color] {
                    self.spec(color, path);
                }
            }
            NodeKind::Annotation {
                font,
                gloss,
                fresh,
                faint,
                pos,
                separator,
                stroke,
                ..
            } => {
                self.font(font, path);
                self.stroke(stroke.as_ref(), path);
                for color in [gloss, fresh, faint]
                    .into_iter()
                    .chain(pos)
                    .chain(separator)
                {
                    self.spec(color, path);
                }
            }
            NodeKind::Use { component } | NodeKind::Repeat { component, .. } => {
                if !self.file.components.contains_key(component) {
                    self.found
                        .push(format!("{path}: 组件 {component:?} 不存在"));
                }
            }
        }
    }

    fn stroke(&mut self, stroke: Option<&BorderSpec>, path: &str) {
        if let Some(stroke) = stroke {
            self.spec(&stroke.color, path);
        }
    }

    fn spec(&mut self, spec: &ColorSpec, path: &str) {
        for color in spec.refs() {
            self.color(color, path);
        }
    }

    fn color(&mut self, color: &ColorRef, path: &str) {
        if let ColorRef::Variable(name) = color
            && !self.file.variables.contains_key(name)
        {
            self.found.push(format!("{path}: 颜色变量 @{name} 不存在"));
        }
    }

    fn font(&mut self, font: &FontRef, path: &str) {
        if let Some(font) = font.base()
            && !self.file.text.styles.contains_key(font)
        {
            self.found.push(format!("{path}: 文字样式 {font:?} 不存在"));
        }
    }
}
