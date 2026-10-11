//! 注音符號元件：聲母 / 介音 / 韻母 / 聲調，鍵盤排列把按鍵解成它。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Component {
    Initial(char),
    Medial(char),
    Final(char),
    Tone(u8, char),
}

impl Component {
    pub fn is_tone(&self) -> bool {
        matches!(self, Component::Tone(_, _))
    }

    pub fn display_char(&self) -> char {
        match self {
            Component::Initial(c) => *c,
            Component::Medial(c) => *c,
            Component::Final(c) => *c,
            Component::Tone(_, c) => *c,
        }
    }
}
