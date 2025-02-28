use rdev::Key;

// 触发类型枚举
#[derive(Debug, Clone)]
pub enum TriggerType {
    SingleKey(Key),
    KeySequence {
        keys: Vec<Key>,
        timeout_ms: u64,
    },
    KeyModifier {
        modifier: Key,
        key: Key,
    },
    Manual,
}