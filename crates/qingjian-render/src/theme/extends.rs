//! 主題繼承：`"extends": "qingjian"` 以那個內建主題為底，本檔案寫的覆蓋上去。在 JSON 層合併：
//! 物件逐鍵合併（`variables` 只寫要改的幾個顏色即可），陣列與標量整個替換（`children` 寫了就是整組換掉）。

use serde_json::{Map, Value};

use super::{ThemeError, jsonc};

/// 繼承鏈最多幾層，擋住互相繼承。
const MAX_DEPTH: usize = 4;

/// 展開 `extends`，返回合併後的整份主題。`base` 按 id 給出內建主題的原始檔。
/// 沒寫 `extends` 的一律以 `root`（青簡綠）為底，內建主題與繼承鏈上的每一層都一樣，只寫幾個顏色也能用；
/// `root` 自己是最底層，不再往下找（不然自己繼承自己）。
pub(super) fn resolve(
    json: &str,
    base: &dyn Fn(&str) -> Option<&'static str>,
    root: &str,
) -> Result<Value, ThemeError> {
    let value: Value = serde_json::from_str(&jsonc::strip(json))?;
    let own = value
        .pointer("/meta/id")
        .and_then(Value::as_str)
        .map(str::to_owned);
    resolve_value(value, own.as_deref(), base, root, 0)
}

/// 主題檔案頂層 `extends` 寫的 id（只看這一層）；沒寫或讀不出來為 `None`。
pub(super) fn base_id(json: &str) -> Option<String> {
    let value: Value = serde_json::from_str(&jsonc::strip(json)).ok()?;
    value.get("extends")?.as_str().map(str::to_owned)
}

/// `own` 是這一層主題自己的 id（頂層取 `meta.id`，往下是正在展開的內建主題）。
fn resolve_value(
    mut value: Value,
    own: Option<&str>,
    base: &dyn Fn(&str) -> Option<&'static str>,
    root: &str,
    depth: usize,
) -> Result<Value, ThemeError> {
    let written = value
        .as_object_mut()
        .and_then(|object| object.remove("extends"));
    let id = match written {
        Some(id) => id,
        None if own == Some(root) => return Ok(value),
        None => Value::String(root.to_owned()),
    };
    let Value::String(id) = id else {
        return Err(ThemeError::UnknownBase { id: id.to_string() });
    };
    if depth >= MAX_DEPTH {
        return Err(ThemeError::TooDeep { id });
    }
    let source = base(&id).ok_or_else(|| ThemeError::UnknownBase { id: id.clone() })?;
    let mut merged = resolve_value(
        serde_json::from_str(&jsonc::strip(source))?,
        Some(&id),
        base,
        root,
        depth + 1,
    )?;
    merge(&mut merged, value);
    Ok(merged)
}

/// 把 `over` 合併進 `base`：兩邊都是物件就逐鍵遞迴，否則整個替換。
fn merge(base: &mut Value, over: Value) {
    match (base, over) {
        (Value::Object(base), Value::Object(over)) => merge_objects(base, over),
        (base, over) => *base = over,
    }
}

fn merge_objects(base: &mut Map<String, Value>, over: Map<String, Value>) {
    for (key, value) in over {
        match base.get_mut(&key) {
            Some(existing) => merge(existing, value),
            None => {
                base.insert(key, value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = r##"{ "a": { "x": 1, "y": [1, 2] }, "b": "keep" }"##;

    fn base(id: &str) -> Option<&'static str> {
        match id {
            "base" => Some(BASE),
            "loop" => Some(r#"{ "extends": "loop" }"#),
            "middle" => Some(r#"{ "m": 1 }"#),
            _ => None,
        }
    }

    #[test]
    fn merges_objects_and_replaces_arrays() {
        let merged = resolve(
            r#"{ "extends": "base", "a": { "y": [3] }, "c": 1 }"#,
            &base,
            "base",
        )
        .unwrap();
        assert_eq!(
            merged,
            serde_json::json!({ "a": { "x": 1, "y": [3] }, "b": "keep", "c": 1 })
        );
    }

    #[test]
    fn rejects_unknown_and_cyclic_bases() {
        assert!(matches!(
            resolve(r#"{ "extends": "nope" }"#, &base, "base"),
            Err(ThemeError::UnknownBase { .. })
        ));
        assert!(matches!(
            resolve(r#"{ "extends": "loop" }"#, &base, "base"),
            Err(ThemeError::TooDeep { .. })
        ));
    }

    #[test]
    fn missing_extends_falls_back_to_root_at_every_level() {
        // 頂層沒寫
        let top = resolve(r#"{ "c": 1 }"#, &base, "base").unwrap();
        assert_eq!(top["b"], "keep");
        // 中間一層沒寫（內建主題忘了寫）也接到最底層
        let middle = resolve(r#"{ "extends": "middle" }"#, &base, "base").unwrap();
        assert_eq!(
            (&middle["m"], &middle["b"]),
            (&serde_json::json!(1), &serde_json::json!("keep"))
        );
        // 最底層自己不再往下找
        let root = resolve(r#"{ "meta": { "id": "base" }, "z": 0 }"#, &base, "base").unwrap();
        assert!(root.get("b").is_none());
    }
}
