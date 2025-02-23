use chrono::NaiveDateTime;
use serde::{Deserialize, Deserializer, Serializer};

/// 对非可选 `NaiveDateTime` 进行秒级时间戳序列化
pub mod naive {
    use super::*;

    pub fn serialize<S>(dt: &NaiveDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // chrono 建议别再调用 dt.timestamp()，改用 dt.and_utc().timestamp()
        let sec = dt.and_utc().timestamp();
        serializer.serialize_i64(sec)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let sec = i64::deserialize(deserializer)?;
        // `NaiveDateTime::from_timestamp` 已被标记 deprecated，但目前仍可用。
        // 如果你不想看到警告，可在此加 `#[allow(deprecated)]`.
        #[allow(deprecated)]
        let dt = NaiveDateTime::from_timestamp(sec, 0);
        Ok(dt)
    }
}

/// 针对可选 `Option<NaiveDateTime>` 
pub mod naive_opt {
    use super::*;

    pub fn serialize<S>(maybe_dt: &Option<NaiveDateTime>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match maybe_dt {
            Some(dt) => {
                let sec = dt.and_utc().timestamp();
                serializer.serialize_some(&sec)
            }
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<NaiveDateTime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt_sec = Option::<i64>::deserialize(deserializer)?;
        match opt_sec {
            Some(sec) => {
                #[allow(deprecated)]
                let dt = NaiveDateTime::from_timestamp(sec, 0);
                Ok(Some(dt))
            }
            None => Ok(None),
        }
    }
}