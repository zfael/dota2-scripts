pub mod binding;
pub mod click_hold;
pub mod keyboard;
pub mod simulation;

pub use binding::{KeyBinding, MouseBinding, NamedKey, PressableKey};
pub use simulation::{press_binding, press_key};