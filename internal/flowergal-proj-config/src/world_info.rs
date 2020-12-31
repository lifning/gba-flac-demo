use crate::resources::ColorEffectType;
use crate::sound_info::{MusicId, TrackList};

pub struct WorldResourceInfo {
    pub id: WorldId,
    pub name: &'static str,
    pub tilemap_path: &'static str,
    pub skybox_path: Option<&'static str>,
    pub effect_path: Option<&'static str>,
    pub effect_type: ColorEffectType,
    pub anim_path: Option<&'static str>,
    pub minimap_path: Option<&'static str>,
    pub songs: TrackList,
}

// indeces in the WORLD_INFO array, among others
#[repr(usize)]
#[derive(Ord, PartialOrd, Eq, PartialEq, Copy, Clone)]
#[cfg_attr(not(target_arch = "arm"), derive(Debug))]
pub enum WorldId {
    TomsDiner,
}

pub const WORLD_RESOURCE_INFO: &[WorldResourceInfo] = &[
    WorldResourceInfo {
        id: WorldId::TomsDiner,
        name: "TOMS_DINER",
        tilemap_path: "aaker-4gvSHtHgOx4-unsplash.png", // https://unsplash.com/photos/4gvSHtHgOx4
        skybox_path: Some("amanda-dalbjorn-fvInY-Gh7sc-unsplash.png"), // https://unsplash.com/photos/fvInY-Gh7sc
        effect_path: Some("overlay.png"),
        effect_type: ColorEffectType::Overlay,
        anim_path: None,
        minimap_path: Some("Minimap_Apartment.csv"),
        songs: TrackList(&[MusicId::TomsDiner]),
    },
];
