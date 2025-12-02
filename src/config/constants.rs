use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
    pub static ref SLOT_KEY_MAPPING: HashMap<&'static str, char> = {
        let mut m = HashMap::new();
        m.insert("slot0", 'z');
        m.insert("slot1", 'x');
        m.insert("slot2", 'c');
        m.insert("slot3", 'v');
        m.insert("slot4", 'b');
        m.insert("slot5", 'n');
        m.insert("neutral0", '0');
        m
    };
}
