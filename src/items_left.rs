//! Time left over items of about the same cost (frames, tiles, passes):
//! shared by the windows' estimator (`app::eta`) and the command line.

/// The progress within a stage at which its measurement and its prior
/// count the same (see [`blend`]).
pub const BLEND_HALF: f64 = 0.08;

/// A stage's whole time: its measured one and its prior (what it took
/// last time, or the model's) blended by the progress `g` (0..1) it made,
/// half and half at [`BLEND_HALF`]; the one of them that exists when the
/// other is missing (None without both).
pub fn blend(measured: Option<f64>, prior: Option<f64>, g: f64) -> Option<f64> {
    match (measured, prior) {
        (Some(m), Some(p)) => {
            let a = g.clamp(0.0, 1.0) / (g.clamp(0.0, 1.0) + BLEND_HALF);
            Some(a * m + (1.0 - a) * p)
        }
        (m, p) => m.or(p),
    }
}

/// Items of about the same cost one after another (Monte Carlo passes,
/// animation frames, tiles): the time each takes from those finished, and
/// the time left with the running item's own estimate.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ItemsLeft {
    per: Option<f64>,
    n: u32,
}

impl ItemsLeft {
    /// An item finished after `secs`: averaged with the last few.
    pub fn done(&mut self, secs: f64) {
        self.n += 1;
        let w = 1.0 / self.n.min(4) as f64;
        self.per = Some(self.per.map_or(secs, |p| p + (secs - p) * w));
    }

    /// The time an item takes (None before one finished).
    pub fn per_item(&self) -> Option<f64> {
        self.per
    }

    /// The time left: the running item's (its progress `g`, its time so far
    /// `secs` and its measured whole time `measured`, None without one,
    /// blended with the finished items' time) and `after` items more (None
    /// while nothing says how long an item takes).
    pub fn left(&self, g: f64, secs: f64, measured: Option<f64>, after: usize) -> Option<f64> {
        let total = blend(measured, self.per, g)?;
        Some((total - secs).max(0.0) + after as f64 * self.per.unwrap_or(total))
    }
}
