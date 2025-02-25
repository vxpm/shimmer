use crate::vram::{VRAM_HEIGHT, VRAM_WIDTH};
use bitvec::BitArr;

const DIRTY_REGION_LEN: u16 = 32;
type Regions = BitArr!(for ((1024 / DIRTY_REGION_LEN) * (1024 / DIRTY_REGION_LEN)) as usize);

#[derive(Debug, Clone, Copy, Default)]
pub struct Region {
    top_left: (u16, u16),
    dimensions: (u16, u16),
}

impl Region {
    pub fn new(top_left: (u16, u16), dimensions: (u16, u16)) -> Self {
        Self {
            top_left,
            dimensions,
        }
    }

    pub fn from_extremes(top_left: (u16, u16), bottom_right: (u16, u16)) -> Self {
        Self {
            top_left,
            dimensions: (bottom_right.0 - top_left.0, bottom_right.1 - top_left.1),
        }
    }
}

/// Helper struct for keeping track of dirty VRAM regions.
#[derive(Debug, Default)]
pub struct DirtyRegions {
    regions: Regions,
}

impl DirtyRegions {
    /// Marks a rectangular region in VRAM as dirty.
    pub fn mark(&mut self, region: Region) {
        if region.dimensions.0 == 0 || region.dimensions.1 == 0 {
            return;
        }

        let start_x = region.top_left.0 / DIRTY_REGION_LEN;
        let end_x = (region.top_left.0 + region.dimensions.0 - 1) / DIRTY_REGION_LEN;
        let start_y = (region.top_left.1) / DIRTY_REGION_LEN;
        let end_y = (region.top_left.1 + region.dimensions.1 - 1) / DIRTY_REGION_LEN;

        for y in start_y..=end_y {
            for x in start_x..=end_x {
                let x = x % (VRAM_WIDTH / DIRTY_REGION_LEN);
                let y = y % (VRAM_HEIGHT / DIRTY_REGION_LEN);

                let bit = self
                    .regions
                    .get_mut((y * (VRAM_WIDTH / DIRTY_REGION_LEN) + x) as usize);

                if let Some(mut bit) = bit {
                    *bit = true;
                }
            }
        }
    }

    /// Unmarks all dirty regions.
    pub fn clear(&mut self) {
        self.regions.fill(false);
    }

    /// Checks whether a given rectangular region in VRAM is dirty.
    pub fn is_dirty(&mut self, region: Region) -> bool {
        if region.dimensions.0 == 0 || region.dimensions.1 == 0 {
            return false;
        }

        let start_x = region.top_left.0 / DIRTY_REGION_LEN;
        let end_x = (region.top_left.0 + region.dimensions.0 - 1) / DIRTY_REGION_LEN;
        let start_y = (region.top_left.1) / DIRTY_REGION_LEN;
        let end_y = (region.top_left.1 + region.dimensions.1 - 1) / DIRTY_REGION_LEN;

        for y in start_y..=end_y {
            for x in start_x..=end_x {
                let x = x % (VRAM_WIDTH / DIRTY_REGION_LEN);
                let y = y % (VRAM_HEIGHT / DIRTY_REGION_LEN);

                if self
                    .regions
                    .get((y * (VRAM_WIDTH / DIRTY_REGION_LEN) + x) as usize)
                    .map(|bit| *bit)
                    .unwrap()
                {
                    return true;
                }
            }
        }

        false
    }
}
