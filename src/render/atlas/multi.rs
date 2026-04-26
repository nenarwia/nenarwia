use std::collections::{HashMap, HashSet};

use super::TextureAtlas;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ThumbTier {
    Px32 = 0,
    Px64 = 1,
    Px128 = 2,
    Px256 = 3,
    Px512 = 4,
}

impl ThumbTier {
    pub fn index(self) -> usize {
        self as usize
    }

    pub fn page_size(self) -> u32 {
        match self {
            ThumbTier::Px32 => 32,
            ThumbTier::Px64 => 64,
            ThumbTier::Px128 => 128,
            ThumbTier::Px256 => 256,
            ThumbTier::Px512 => 512,
        }
    }

    pub fn from_page_size(px: u32) -> Option<Self> {
        match px {
            32 => Some(ThumbTier::Px32),
            64 => Some(ThumbTier::Px64),
            128 => Some(ThumbTier::Px128),
            256 => Some(ThumbTier::Px256),
            512 => Some(ThumbTier::Px512),
            _ => None,
        }
    }

    /// Encode tier into atlas-UV's X channel as: x = tier + u.
    /// This keeps InstanceRaw unchanged (no extra per-instance attribute needed).
    pub fn encode_uv_x(self, u: f32) -> f32 {
        (self as u8 as f32) + u
    }

    pub fn decode_uv_x(x: f32) -> Option<Self> {
        if !x.is_finite() {
            return None;
        }
        let t = x.floor() as i32;
        if !(0..=4).contains(&t) {
            return None;
        }
        match t {
            0 => Some(ThumbTier::Px32),
            1 => Some(ThumbTier::Px64),
            2 => Some(ThumbTier::Px128),
            3 => Some(ThumbTier::Px256),
            4 => Some(ThumbTier::Px512),
            _ => None,
        }
    }
}

pub struct UploadResult {
    pub uv_region: [f32; 4],
    pub evicted: Option<u64>,
}

pub struct ThumbnailUploadInput<'a> {
    pub tier: ThumbTier,
    pub queue: &'a wgpu::Queue,
    pub id: u64,
    pub data: &'a [u8],
    pub frame: u64,
    pub class: ThumbClass,
    pub visible_ids: &'a HashSet<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ThumbClass {
    Coverage,
    Quality,
}

impl ThumbClass {
    #[inline]
    fn eviction_rank(self) -> u8 {
        match self {
            ThumbClass::Quality => 0,
            ThumbClass::Coverage => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AtlasUsage {
    pub used_slots: u32,
    pub total_slots: u32,
    pub unique_ids: u32,
    pub used_bytes: u64,
    pub total_bytes: u64,
}

/// Multi-tier thumbnail atlases (Variant 1).
///
/// - Each tier has its own atlas texture (same format), and its own LRU allocator.
/// - The shader selects which atlas to sample using tier encoded in UV.x (tier + u).
pub struct MultiTierAtlas {
    tiers: [TextureAtlas; 5],
    enabled: [bool; 5],
    id_to_slot: [HashMap<u64, u32>; 5],
    id_to_class: [HashMap<u64, ThumbClass>; 5],
}

impl MultiTierAtlas {
    /// `dims` is per-tier atlas texture dimension (in pixels). Use 0 to disable a tier.
    /// Disabled tiers still allocate a minimal texture (page_size) so bindings stay valid.
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, dims: [u32; 5], max_dim: u32) -> Self {
        let mut enabled = [false; 5];

        let mut make = |tier: ThumbTier, dim: u32| -> TextureAtlas {
            let ps = tier.page_size();
            let actual_dim = if dim == 0 { ps } else { dim };
            enabled[tier.index()] = dim != 0;
            TextureAtlas::new_with_page_size(device, queue, actual_dim, max_dim, ps)
        };

        let t0 = make(ThumbTier::Px32, dims[0]);
        let t1 = make(ThumbTier::Px64, dims[1]);
        let t2 = make(ThumbTier::Px128, dims[2]);
        let t3 = make(ThumbTier::Px256, dims[3]);
        let t4 = make(ThumbTier::Px512, dims[4]);

        Self {
            tiers: [t0, t1, t2, t3, t4],
            enabled,
            id_to_slot: [
                HashMap::new(),
                HashMap::new(),
                HashMap::new(),
                HashMap::new(),
                HashMap::new(),
            ],
            id_to_class: [
                HashMap::new(),
                HashMap::new(),
                HashMap::new(),
                HashMap::new(),
                HashMap::new(),
            ],
        }
    }

    pub fn enabled(&self, tier: ThumbTier) -> bool {
        self.enabled[tier.index()]
    }

    pub fn views(&self) -> [&wgpu::TextureView; 5] {
        [
            &self.tiers[0].gpu.view,
            &self.tiers[1].gpu.view,
            &self.tiers[2].gpu.view,
            &self.tiers[3].gpu.view,
            &self.tiers[4].gpu.view,
        ]
    }

    /// We keep shared samplers (all tiers share identical sampler settings).
    pub fn sampler_linear(&self) -> &wgpu::Sampler {
        &self.tiers[0].gpu.sampler_linear
    }

    pub fn sampler_nearest(&self) -> &wgpu::Sampler {
        &self.tiers[0].gpu.sampler_nearest
    }

    pub fn touch(&mut self, tier: ThumbTier, id: u64, frame: u64) {
        let idx = tier.index();
        if let Some(&slot) = self.id_to_slot[idx].get(&id) {
            self.tiers[idx].touch_slot(slot, frame);
        }
    }

    pub fn has(&self, tier: ThumbTier, id: u64) -> bool {
        self.id_to_slot[tier.index()].contains_key(&id)
    }

    pub fn uv_for_id_tier(&self, id: u64, tier: ThumbTier) -> Option<[f32; 4]> {
        let idx = tier.index();
        let slot = *self.id_to_slot[idx].get(&id)?;
        let mut uv = self.tiers[idx].uv_for_slot(slot);
        uv[0] = tier.encode_uv_x(uv[0]);
        Some(uv)
    }

    pub fn upload_thumbnail(&mut self, input: ThumbnailUploadInput<'_>) -> Option<UploadResult> {
        let tier = input.tier;
        let queue = input.queue;
        let id = input.id;
        let data = input.data;
        let frame = input.frame;
        let class = input.class;
        let visible_ids = input.visible_ids;
        let idx = tier.index();

        if let Some(&existing_slot) = self.id_to_slot[idx].get(&id) {
            self.id_to_class[idx].insert(id, class);
            let mut uv = self.tiers[idx].upload_to_slot(queue, existing_slot, id, data, frame);
            uv[0] = tier.encode_uv_x(uv[0]);
            return Some(UploadResult {
                uv_region: uv,
                evicted: None,
            });
        }

        let class_map = &self.id_to_class[idx];
        let (slot_opt, evicted) = self.tiers[idx].allocate_lru_with_policy(frame, |owner| {
            let Some(owner_id) = owner else {
                return Some(ThumbClass::Quality.eviction_rank());
            };
            let owner_class = class_map
                .get(&owner_id)
                .copied()
                .unwrap_or(ThumbClass::Quality);
            if owner_class == ThumbClass::Coverage && visible_ids.contains(&owner_id) {
                return None;
            }
            Some(owner_class.eviction_rank())
        });
        let slot = slot_opt?;

        if let Some(eid) = evicted {
            self.id_to_slot[idx].remove(&eid);
            self.id_to_class[idx].remove(&eid);
        }

        self.id_to_slot[idx].insert(id, slot);
        self.id_to_class[idx].insert(id, class);

        let mut uv = self.tiers[idx].upload_to_slot(queue, slot, id, data, frame);
        uv[0] = tier.encode_uv_x(uv[0]);

        Some(UploadResult {
            uv_region: uv,
            evicted,
        })
    }

    pub fn best_available_uv(&self, id: u64) -> Option<(ThumbTier, [f32; 4])> {
        let tiers = [
            ThumbTier::Px512,
            ThumbTier::Px256,
            ThumbTier::Px128,
            ThumbTier::Px64,
            ThumbTier::Px32,
        ];

        for tier in tiers.iter().copied() {
            if !self.enabled(tier) {
                continue;
            }
            if let Some(&slot) = self.id_to_slot[tier.index()].get(&id) {
                let mut uv = self.tiers[tier.index()].uv_for_slot(slot);
                uv[0] = tier.encode_uv_x(uv[0]);
                return Some((tier, uv));
            }
        }
        None
    }

    pub fn remove_id(&mut self, queue: &wgpu::Queue, id: u64) -> bool {
        let mut removed = false;
        for tier in [
            ThumbTier::Px32,
            ThumbTier::Px64,
            ThumbTier::Px128,
            ThumbTier::Px256,
            ThumbTier::Px512,
        ] {
            let idx = tier.index();
            if self.id_to_slot[idx].remove(&id).is_some() {
                self.id_to_class[idx].remove(&id);
                removed |= self.tiers[idx].remove_id(queue, id);
            }
        }
        removed
    }

    pub fn clear(&mut self) {
        for tier in &mut self.tiers {
            tier.clear();
        }
        for map in &mut self.id_to_slot {
            map.clear();
        }
        for map in &mut self.id_to_class {
            map.clear();
        }
    }

    pub fn usage(&self) -> AtlasUsage {
        let mut usage = AtlasUsage::default();
        let mut unique_ids =
            HashSet::with_capacity(self.id_to_slot.iter().map(HashMap::len).sum::<usize>());
        for (idx, tier) in self.tiers.iter().enumerate() {
            let total_slots = tier.columns * tier.columns;
            let used_slots = self.id_to_slot[idx].len() as u32;
            let page_bytes = (tier.page_size as u64) * (tier.page_size as u64) * 4;
            unique_ids.extend(self.id_to_slot[idx].keys().copied());

            usage.used_slots = usage.used_slots.saturating_add(used_slots);
            usage.total_slots = usage.total_slots.saturating_add(total_slots);
            usage.used_bytes = usage
                .used_bytes
                .saturating_add(used_slots as u64 * page_bytes);
            usage.total_bytes = usage
                .total_bytes
                .saturating_add(total_slots as u64 * page_bytes);
        }
        usage.unique_ids = if unique_ids.len() > u32::MAX as usize {
            u32::MAX
        } else {
            unique_ids.len() as u32
        };
        usage
    }
}
