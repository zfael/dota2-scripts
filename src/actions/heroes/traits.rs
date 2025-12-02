use crate::models::GsiWebhookEvent;
use std::any::Any;

/// Trait for hero-specific automation scripts
pub trait HeroScript: Send + Sync {
    /// Handle GSI event for hero-specific automations
    fn handle_gsi_event(&self, event: &GsiWebhookEvent);

    /// Handle standalone combo trigger (e.g., HOME key press)
    fn handle_standalone_trigger(&self);

    /// Get hero name for dispatcher routing
    fn hero_name(&self) -> &'static str;
    
    /// Allow downcasting to concrete types
    fn as_any(&self) -> &dyn Any;
}
