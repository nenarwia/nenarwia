use super::region::{PtRegion, PT_TEXTURE_SIZE};
pub struct FreeRectAllocator {
    free: Vec<PtRegion>,
}

impl FreeRectAllocator {
    pub fn new_full() -> Self {
        Self {
            free: vec![PtRegion {
                x: 0,
                y: 0,
                w: PT_TEXTURE_SIZE,
                h: PT_TEXTURE_SIZE,
            }],
        }
    }

    pub fn reset_full(&mut self) {
        self.free.clear();
        self.free.push(PtRegion {
            x: 0,
            y: 0,
            w: PT_TEXTURE_SIZE,
            h: PT_TEXTURE_SIZE,
        });
    }

    pub fn alloc(&mut self, w: u32, h: u32) -> Option<PtRegion> {
        if w == 0 || h == 0 || w > PT_TEXTURE_SIZE || h > PT_TEXTURE_SIZE {
            return None;
        }
        let mut best_i: Option<usize> = None;
        let mut best_score: u64 = u64::MAX;

        for (i, r) in self.free.iter().enumerate() {
            if r.w >= w && r.h >= h {
                let waste = (r.w - w) as u64 * (r.h - h) as u64;
                if waste < best_score {
                    best_score = waste;
                    best_i = Some(i);
                }
            }
        }

        let i = best_i?;
        let r = self.free.swap_remove(i);
        let out = PtRegion {
            x: r.x,
            y: r.y,
            w,
            h,
        };

        // Guillotine split: right + bottom.
        let rw = r.w.saturating_sub(w);
        let bh = r.h.saturating_sub(h);
        if rw > 0 {
            self.free.push(PtRegion {
                x: r.x + w,
                y: r.y,
                w: rw,
                h,
            });
        }
        if bh > 0 {
            self.free.push(PtRegion {
                x: r.x,
                y: r.y + h,
                w: r.w,
                h: bh,
            });
        }

        Some(out)
    }

    pub fn free_rect(&mut self, rect: PtRegion) {
        if rect.w == 0 || rect.h == 0 {
            return;
        }
        self.free.push(rect);
        self.merge_adjacent();
    }

    fn merge_adjacent(&mut self) {
        let mut changed = true;
        while changed {
            changed = false;
            'outer: for i in 0..self.free.len() {
                for j in (i + 1)..self.free.len() {
                    let a = self.free[i];
                    let b = self.free[j];

                    // Horizontal merge: same y/h and touching on x.
                    if a.y == b.y && a.h == b.h {
                        if a.x + a.w == b.x {
                            self.free[i] = PtRegion {
                                x: a.x,
                                y: a.y,
                                w: a.w + b.w,
                                h: a.h,
                            };
                            self.free.swap_remove(j);
                            changed = true;
                            break 'outer;
                        }
                        if b.x + b.w == a.x {
                            self.free[i] = PtRegion {
                                x: b.x,
                                y: a.y,
                                w: a.w + b.w,
                                h: a.h,
                            };
                            self.free.swap_remove(j);
                            changed = true;
                            break 'outer;
                        }
                    }

                    // Vertical merge: same x/w and touching on y.
                    if a.x == b.x && a.w == b.w {
                        if a.y + a.h == b.y {
                            self.free[i] = PtRegion {
                                x: a.x,
                                y: a.y,
                                w: a.w,
                                h: a.h + b.h,
                            };
                            self.free.swap_remove(j);
                            changed = true;
                            break 'outer;
                        }
                        if b.y + b.h == a.y {
                            self.free[i] = PtRegion {
                                x: a.x,
                                y: b.y,
                                w: a.w,
                                h: a.h + b.h,
                            };
                            self.free.swap_remove(j);
                            changed = true;
                            break 'outer;
                        }
                    }
                }
            }
        }
    }
}
