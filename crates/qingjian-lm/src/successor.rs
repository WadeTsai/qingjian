use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// CSR 裡的一條後繼：後詞編號與 (前詞, 後詞) 計數。8 位元組，原樣落盤。
#[derive(Debug, Clone, Copy, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub struct Successor {
    /// 後詞編號。
    pub word: u32,

    /// 二元計數。
    pub count: u32,
}
