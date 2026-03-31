/// A region of the Dota 2 minimap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapZone {
    TopLane,
    MidLane,
    BotLane,
    DireJungle,
    RadiantJungle,
    Roshan,
    Other,
}

impl std::fmt::Display for MapZone {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MapZone::TopLane => write!(f, "Top Lane"),
            MapZone::MidLane => write!(f, "Mid Lane"),
            MapZone::BotLane => write!(f, "Bot Lane"),
            MapZone::DireJungle => write!(f, "Dire Jungle"),
            MapZone::RadiantJungle => write!(f, "Radiant Jungle"),
            MapZone::Roshan => write!(f, "Roshan"),
            MapZone::Other => write!(f, "Other"),
        }
    }
}

struct ZoneBounds {
    zone: MapZone,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
}

// Roshan is checked first because its bounds overlap with MidLane.
const ZONE_DEFS: [ZoneBounds; 6] = [
    ZoneBounds { zone: MapZone::Roshan,        x1: 0.30, y1: 0.32, x2: 0.45, y2: 0.45 },
    ZoneBounds { zone: MapZone::TopLane,        x1: 0.00, y1: 0.00, x2: 0.25, y2: 0.55 },
    ZoneBounds { zone: MapZone::BotLane,        x1: 0.75, y1: 0.45, x2: 1.00, y2: 1.00 },
    ZoneBounds { zone: MapZone::DireJungle,     x1: 0.45, y1: 0.00, x2: 1.00, y2: 0.45 },
    ZoneBounds { zone: MapZone::RadiantJungle,  x1: 0.00, y1: 0.55, x2: 0.55, y2: 1.00 },
    ZoneBounds { zone: MapZone::MidLane,        x1: 0.25, y1: 0.25, x2: 0.75, y2: 0.75 },
];

/// Classify a pixel coordinate into a map zone.
///
/// Coordinates are pixel positions within the capture region (0-indexed).
/// The position is normalized to `[0.0, 1.0]` and checked against predefined
/// zone rectangles. Returns `MapZone::Other` if the point doesn't match any zone.
pub fn classify_zone(x: u32, y: u32, image_width: u32, image_height: u32) -> MapZone {
    if image_width == 0 || image_height == 0 {
        return MapZone::Other;
    }
    let nx = x as f32 / image_width as f32;
    let ny = y as f32 / image_height as f32;
    for def in &ZONE_DEFS {
        if nx >= def.x1 && nx <= def.x2 && ny >= def.y1 && ny <= def.y2 {
            return def.zone;
        }
    }
    MapZone::Other
}
