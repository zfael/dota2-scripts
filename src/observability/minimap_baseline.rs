/// Accumulates color masks from N frames and identifies consistently-present
/// pixels as static map fixtures (towers, buildings, camp markers).
pub struct BaselineMask {
    #[allow(dead_code)]
    width: u32,
    #[allow(dead_code)]
    height: u32,
    red_counts: Vec<u32>,
    green_counts: Vec<u32>,
    frames: u32,
    threshold: f32,
    built: bool,
    static_red: Vec<bool>,
    static_green: Vec<bool>,
}

impl BaselineMask {
    /// Create a new baseline accumulator for the given image dimensions.
    ///
    /// `threshold` is the fraction of frames a pixel must appear in to be
    /// marked as static (e.g., 0.8 = must appear in ≥80% of frames).
    pub fn new(width: u32, height: u32, threshold: f32) -> Self {
        let total = (width * height) as usize;
        Self {
            width,
            height,
            red_counts: vec![0; total],
            green_counts: vec![0; total],
            frames: 0,
            threshold,
            built: false,
            static_red: Vec::new(),
            static_green: Vec::new(),
        }
    }

    /// Feed one frame's red and green boolean masks into the accumulator.
    pub fn accumulate_frame(&mut self, red_mask: &[bool], green_mask: &[bool]) {
        let total = (self.width * self.height) as usize;
        for i in 0..total.min(red_mask.len()) {
            if red_mask[i] {
                self.red_counts[i] += 1;
            }
        }
        for i in 0..total.min(green_mask.len()) {
            if green_mask[i] {
                self.green_counts[i] += 1;
            }
        }
        self.frames += 1;
    }

    /// Finalize the baseline after all frames have been accumulated.
    ///
    /// Pixels appearing in ≥ `threshold` fraction of frames are marked static.
    pub fn build(&mut self) {
        if self.frames == 0 {
            self.built = true;
            return;
        }
        let cutoff = (self.frames as f32 * self.threshold) as u32;
        let total = (self.width * self.height) as usize;
        self.static_red = self.red_counts.iter().take(total).map(|&c| c >= cutoff).collect();
        self.static_green = self.green_counts.iter().take(total).map(|&c| c >= cutoff).collect();
        self.built = true;
    }

    /// Whether `build()` has been called.
    pub fn is_built(&self) -> bool {
        self.built
    }

    /// Whether pixel at `idx` is a static red element.
    pub fn is_static_red(&self, idx: usize) -> bool {
        self.static_red.get(idx).copied().unwrap_or(false)
    }

    /// Whether pixel at `idx` is a static green element.
    pub fn is_static_green(&self, idx: usize) -> bool {
        self.static_green.get(idx).copied().unwrap_or(false)
    }

    /// How many frames have been accumulated so far.
    pub fn frame_count(&self) -> u32 {
        self.frames
    }
}
