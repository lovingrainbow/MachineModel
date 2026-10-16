//! 拒絕重複鍵的映射，以及 `ext` 擴充欄位的型別。
//!
//! serde 對 struct 的重複欄位本來就會報錯，但對 map 的重複鍵會讓後者靜默覆蓋前者。
//! `ext` 與 `devices[].bind` 是 map，所以用 [`StrictMap`] 補上這個缺口。

use std::collections::BTreeMap;
use std::fmt;
use std::marker::PhantomData;

use serde::de::{Error, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrictMap<V>(pub BTreeMap<String, V>);

impl<V> Default for StrictMap<V> {
    fn default() -> Self {
        Self(BTreeMap::new())
    }
}

impl<V> StrictMap<V> {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn get(&self, key: &str) -> Option<&V> {
        self.0.get(key)
    }
}

/// 序列化時與普通 map 相同（重複鍵已在讀取端擋下，這裡不需要特別處理）。
impl<V: Serialize> Serialize for StrictMap<V> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de, V: Deserialize<'de>> Deserialize<'de> for StrictMap<V> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct MapVisitor<V>(PhantomData<V>);

        impl<'de, V: Deserialize<'de>> Visitor<'de> for MapVisitor<V> {
            type Value = StrictMap<V>;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a mapping")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
                let mut map = BTreeMap::new();
                while let Some((key, value)) = access.next_entry::<String, V>()? {
                    if map.insert(key.clone(), value).is_some() {
                        return Err(A::Error::custom(format!("重複的鍵 `{key}`")));
                    }
                }
                Ok(StrictMap(map))
            }
        }

        deserializer.deserialize_map(MapVisitor(PhantomData))
    }
}

/// `ext` 內的值：只接受字串、整數、布林。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ExtValue {
    Bool(bool),
    Int(i64),
    Text(String),
}

/// 原樣保存匯入檔中模型不理解的欄位：`ext.<廠牌>.<欄位>`。
pub type Ext = StrictMap<StrictMap<ExtValue>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Deserialize)]
    struct Holder {
        ext: Ext,
    }

    #[test]
    fn accepts_nested_scalars() {
        let h: Holder = serde_yaml_ng::from_str(
            "ext:\n  inovance:\n    seq: 83\n    network_public: 私有\n    flag: true\n",
        )
        .unwrap();
        let inov = h.ext.get("inovance").unwrap();
        assert_eq!(inov.get("seq"), Some(&ExtValue::Int(83)));
        assert_eq!(
            inov.get("network_public"),
            Some(&ExtValue::Text("私有".into()))
        );
        assert_eq!(inov.get("flag"), Some(&ExtValue::Bool(true)));
    }

    #[test]
    fn rejects_duplicate_keys_at_both_levels() {
        let outer = serde_yaml_ng::from_str::<Holder>("ext:\n  a:\n    k: 1\n  a:\n    k: 2\n");
        assert!(outer.is_err(), "{outer:?}");
        let inner = serde_yaml_ng::from_str::<Holder>("ext:\n  a:\n    k: 1\n    k: 2\n");
        assert!(inner.is_err(), "{inner:?}");
        assert!(inner.unwrap_err().to_string().contains("重複的鍵"));
    }

    #[test]
    fn yaml_no_stays_a_string() {
        let h: Holder = serde_yaml_ng::from_str("ext:\n  a:\n    k: no\n    j: ON\n").unwrap();
        assert_eq!(
            h.ext.get("a").unwrap().get("k"),
            Some(&ExtValue::Text("no".into()))
        );
        assert_eq!(
            h.ext.get("a").unwrap().get("j"),
            Some(&ExtValue::Text("ON".into()))
        );
    }
}
