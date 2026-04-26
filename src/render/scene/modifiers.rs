use super::data::Scene;
use crate::render::instance::InstanceRaw;

impl Scene {
    pub fn reset_item_texture(&mut self, id: u64) {
        let Some(idx) = self.index_for_id(id) else {
            return;
        };
        if idx >= self.all_items_raw.len() {
            return;
        }
        let raw = &mut self.all_items_raw[idx];
        raw.uv_region = [0.0, 0.0, 0.0, 0.0];
    }

    pub fn update_item_texture(&mut self, id: u64, uv_region: [f32; 4]) {
        let Some(idx) = self.index_for_id(id) else {
            return;
        };
        if idx >= self.all_items_raw.len() {
            return;
        }
        self.all_items_raw[idx].uv_region = uv_region;
    }

    pub fn set_item_dimensions(&mut self, idx: usize, dims: (u32, u32)) {
        if idx >= self.item_dimensions.len() {
            return;
        }
        self.item_dimensions[idx] = dims;
        if idx < self.all_items_raw.len() {
            self.all_items_raw[idx].fit_rect =
                crate::render::instance::InstanceRaw::contain_fit_rect(dims.0, dims.1);
        }
    }

    pub fn clear_item_media_slot(&mut self, idx: usize) -> Option<(u64, u64)> {
        if idx >= self.all_items_raw.len()
            || idx >= self.index_to_id.len()
            || idx >= self.asset_keys.len()
            || idx >= self.item_dimensions.len()
            || idx >= self.quality_debt.len()
            || idx >= self.last_lod.len()
            || idx >= self.display_lod.len()
            || idx >= self.render_lod.len()
            || idx >= self.coarse_lod.len()
        {
            return None;
        }

        let id = self.index_to_id[idx];
        let old_asset_key = self.asset_keys[idx];

        let raw = &mut self.all_items_raw[idx];
        raw.color = [1.0, 1.0, 1.0, 1.0];
        raw.uv_region = [0.0, 0.0, 0.0, 0.0];
        raw.params = [-1.0, -1.0, 0.0, 0.0];
        raw.params2 = [-1.0, -1.0, 0.0, 0.0];
        raw.sample_flags = [0.0, 0.0, 0.0, 0.0];
        raw.fit_rect = InstanceRaw::FULL_SLOT_FIT_RECT;

        self.asset_keys[idx] = 0;
        self.item_dimensions[idx] = (0, 0);
        self.quality_debt[idx] = 0.0;
        self.last_lod[idx] = 0;
        self.display_lod[idx] = u8::MAX;
        self.render_lod[idx] = u8::MAX;
        self.coarse_lod[idx] = u8::MAX;

        self.refresh_asset_key_index(old_asset_key, idx);
        Some((id, old_asset_key))
    }

    pub fn restore_item_media_slot(
        &mut self,
        idx: usize,
        asset_key: u64,
        dims: (u32, u32),
    ) -> Option<u64> {
        if idx >= self.all_items_raw.len()
            || idx >= self.index_to_id.len()
            || idx >= self.asset_keys.len()
            || idx >= self.item_dimensions.len()
            || idx >= self.quality_debt.len()
            || idx >= self.last_lod.len()
            || idx >= self.display_lod.len()
            || idx >= self.render_lod.len()
            || idx >= self.coarse_lod.len()
        {
            return None;
        }

        let id = self.index_to_id[idx];
        let prev_asset_key = self.asset_keys[idx];

        let raw = &mut self.all_items_raw[idx];
        raw.color = [1.0, 1.0, 1.0, 1.0];
        raw.uv_region = [0.0, 0.0, 0.0, 0.0];
        raw.params = [-1.0, -1.0, 0.0, 0.0];
        raw.params2 = [-1.0, -1.0, 0.0, 0.0];
        raw.sample_flags = [0.0, 0.0, 0.0, 0.0];
        raw.fit_rect = InstanceRaw::contain_fit_rect(dims.0, dims.1);

        self.asset_keys[idx] = asset_key;
        self.item_dimensions[idx] = dims;
        self.quality_debt[idx] = 0.0;
        self.last_lod[idx] = 0;
        self.display_lod[idx] = u8::MAX;
        self.render_lod[idx] = u8::MAX;
        self.coarse_lod[idx] = u8::MAX;

        self.refresh_asset_key_index(prev_asset_key, idx);
        if asset_key != 0 {
            self.asset_key_to_index.insert(asset_key, idx);
        }
        Some(id)
    }

    fn refresh_asset_key_index(&mut self, asset_key: u64, removed_idx: usize) {
        if asset_key == 0 {
            return;
        }

        let mapped = self.asset_key_to_index.get(&asset_key).copied();
        if mapped != Some(removed_idx) {
            return;
        }

        if let Some(next_idx) = self
            .asset_keys
            .iter()
            .enumerate()
            .find_map(|(idx, &key)| (idx != removed_idx && key == asset_key).then_some(idx))
        {
            self.asset_key_to_index.insert(asset_key, next_idx);
        } else {
            self.asset_key_to_index.remove(&asset_key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Scene;
    use crate::core::scanner::FileItem;
    use crate::render::instance::InstanceRaw;
    use std::path::PathBuf;

    fn make_file(id: u64) -> FileItem {
        FileItem {
            id,
            asset_key: id + 10,
            path: PathBuf::from(format!("slot_{id}.png")),
            width: 640,
            height: 360,
        }
    }

    #[test]
    fn clear_item_media_slot_preserves_slot_layout_but_resets_runtime_media() {
        let (mut scene, _) = Scene::from_files(vec![make_file(1), make_file(2)]);
        let before_addresses = scene.slot_addresses.clone();
        let before_blocks: Vec<_> = scene
            .layout_blocks
            .iter()
            .map(|block| {
                (
                    block.block_id,
                    block.grid,
                    block.bounds,
                    block.index_start,
                    block.index_len,
                )
            })
            .collect();

        scene.all_items_raw[0].uv_region = [0.2, 0.3, 0.4, 0.5];
        scene.all_items_raw[0].params = [1.0, 2.0, 3.0, 4.0];
        scene.all_items_raw[0].params2 = [5.0, 6.0, 7.0, 8.0];
        scene.all_items_raw[0].sample_flags = [9.0, 10.0, 11.0, 12.0];

        let cleared = scene.clear_item_media_slot(0);

        assert_eq!(cleared, Some((1, 11)));
        assert_eq!(scene.slot_addresses, before_addresses);
        assert_eq!(
            scene
                .layout_blocks
                .iter()
                .map(|block| (
                    block.block_id,
                    block.grid,
                    block.bounds,
                    block.index_start,
                    block.index_len
                ))
                .collect::<Vec<_>>(),
            before_blocks
        );
        assert_eq!(scene.asset_keys[0], 0);
        assert_eq!(scene.item_dimensions[0], (0, 0));
        assert_eq!(scene.quality_debt[0], 0.0);
        assert_eq!(scene.last_lod[0], 0);
        assert_eq!(scene.display_lod[0], u8::MAX);
        assert_eq!(scene.render_lod[0], u8::MAX);
        assert_eq!(scene.coarse_lod[0], u8::MAX);
        assert_eq!(scene.all_items_raw[0].uv_region, [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(scene.all_items_raw[0].params, [-1.0, -1.0, 0.0, 0.0]);
        assert_eq!(scene.all_items_raw[0].params2, [-1.0, -1.0, 0.0, 0.0]);
        assert_eq!(scene.all_items_raw[0].sample_flags, [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(
            scene.all_items_raw[0].fit_rect,
            InstanceRaw::FULL_SLOT_FIT_RECT
        );
    }

    #[test]
    fn restore_item_media_slot_rehydrates_runtime_fields_without_relayout() {
        let (mut scene, _) = Scene::from_files(vec![make_file(1), make_file(2)]);
        let before_addresses = scene.slot_addresses.clone();
        scene.clear_item_media_slot(0);

        let restored = scene.restore_item_media_slot(0, 91, (1920, 1080));

        assert_eq!(restored, Some(1));
        assert_eq!(scene.slot_addresses, before_addresses);
        assert_eq!(scene.asset_keys[0], 91);
        assert_eq!(scene.item_dimensions[0], (1920, 1080));
        assert_eq!(scene.quality_debt[0], 0.0);
        assert_eq!(scene.display_lod[0], u8::MAX);
        assert_eq!(scene.render_lod[0], u8::MAX);
        assert_eq!(scene.coarse_lod[0], u8::MAX);
        assert_eq!(scene.all_items_raw[0].uv_region, [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(scene.all_items_raw[0].params, [-1.0, -1.0, 0.0, 0.0]);
        assert_eq!(scene.all_items_raw[0].params2, [-1.0, -1.0, 0.0, 0.0]);
        assert_eq!(scene.all_items_raw[0].sample_flags, [0.0, 0.0, 0.0, 0.0]);
        assert_ne!(
            scene.all_items_raw[0].fit_rect,
            InstanceRaw::FULL_SLOT_FIT_RECT
        );
    }
}
