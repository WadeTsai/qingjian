//! 能寫進檔案的字串雜湊索引：開放定址表，槽裡放條目編號，鍵本身留在呼叫方的 arena 裡。
//!
//! 雜湊函式必須跨版本、跨機器穩定（寫檔案的程序和讀檔案的程序算出來要一樣），所以不用標準庫或 foldhash
//! 帶隨機種子的雜湊器，用固定的 FNV-1a 64。詞級查表每鍵幾千次，一次是「雜湊 + 一兩次探測 + 一次字串比較」。

/// 空槽。
pub const EMPTY: u32 = u32::MAX;

/// FNV-1a 64 位，再過一遍 MurmurHash3 的 fmix64 終混：FNV 的低位對短的、位元組模式相近的輸入（UTF-8 中文全是
/// 0xE4–0xE9 開頭的三位元組）分佈不好，而開放定址表只用低位選槽，不混一下探測鏈會很長。
pub fn stable_hash(key: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in key.as_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xff51_afd7_ed55_8ccd);
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    hash ^ (hash >> 33)
}

/// 裝 `count` 條要多大的表：2 的冪、至少兩倍條數（裝載率 ≤ 0.5，線性探測平均一兩步）。
pub fn capacity_for(count: usize) -> usize {
    (count.saturating_mul(2)).next_power_of_two().max(16)
}

/// 建表：條目編號 `0..count`，`key_of(id)` 給每條的鍵。鍵重複時後來的探測到已有的鍵會各佔一槽，
/// 查詢返回先放進去的那條；呼叫方應保證鍵唯一。
pub fn build<'a>(count: usize, key_of: impl Fn(u32) -> &'a str) -> Vec<u32> {
    let capacity = capacity_for(count);
    let mask = capacity - 1;
    let mut table = vec![EMPTY; capacity];
    for id in 0..count {
        let id = u32::try_from(id).expect("hash index holds at most u32::MAX entries");
        let mut slot = (stable_hash(key_of(id)) as usize) & mask;
        while table[slot] != EMPTY {
            slot = (slot + 1) & mask;
        }
        table[slot] = id;
    }
    table
}

/// 查詢：表必須是 [`build`] 出來的（容量 2 的冪、至少一個空槽）。
pub fn find<'a>(table: &[u32], key: &str, key_of: impl Fn(u32) -> &'a str) -> Option<u32> {
    if table.is_empty() {
        return None;
    }
    let mask = table.len() - 1;
    let mut slot = (stable_hash(key) as usize) & mask;
    loop {
        let id = table[slot];
        if id == EMPTY {
            return None;
        }
        if key_of(id) == key {
            return Some(id);
        }
        slot = (slot + 1) & mask;
    }
}

/// 表是否合法：容量 2 的冪，且至少有一個空槽（否則 [`find`] 找不存在的鍵會死迴圈）。
pub fn is_valid(table: &[u32], count: usize) -> bool {
    !table.is_empty()
        && table.len().is_power_of_two()
        && table.iter().filter(|&&slot| slot != EMPTY).count() == count
        && table
            .iter()
            .all(|&slot| slot == EMPTY || (slot as usize) < count)
        && table.len() > count
}

#[cfg(test)]
mod tests {
    use super::*;

    const PINNED_EMPTY: u64 = 0xefd0_1f60_ba99_2926;
    const PINNED_KAIFA: u64 = 0x3758_0f72_eb7d_16d3;

    #[test]
    fn finds_every_key_and_rejects_unknown_ones() {
        let keys = ["我", "想", "去", "吃饭", "<s>", "开发", "开放"];
        let table = build(keys.len(), |id| keys[id as usize]);
        assert!(is_valid(&table, keys.len()));
        for (id, key) in keys.iter().enumerate() {
            assert_eq!(find(&table, key, |id| keys[id as usize]), Some(id as u32));
        }
        assert_eq!(find(&table, "没有", |id| keys[id as usize]), None);
        assert_eq!(find(&[], "我", |id| keys[id as usize]), None);
    }

    #[test]
    fn hash_is_stable() {
        // 固定值：換雜湊函式會讓已有的 .qj 檔案全部失效，測試釘住它
        assert_eq!(stable_hash(""), PINNED_EMPTY);
        assert_eq!(stable_hash("开发"), PINNED_KAIFA);
    }
}
