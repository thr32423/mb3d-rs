//! Per-pixel calculation result (`TsiLight5`), the "G-buffer" that the
//! ray marcher produces and the painter consumes.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SiLight {
    /// Surface normal in view space, scaled to +-32767.
    pub normal: [i16; 3],
    /// `RoughZposFine` + `Zpos` as one cardinal: bits 8..31 = z position
    /// (24 bit), bits 0..7 = roughness.  `Zpos` (bits 16..31) >= 32768
    /// marks background.
    pub zpos_fine: u32,
    /// DE step count (dynamic fog), 10 bit, + hard shadow bits.
    pub shadow: u16,
    pub amb_shadow: u16,
    /// Smoothed iteration gradient for colouring, high bit = inside colour.
    pub si_gradient: u16,
    /// Orbit trap colouring value.
    pub otrap: u16,
}

impl Default for SiLight {
    fn default() -> Self {
        SiLight {
            normal: [0; 3],
            zpos_fine: 32768 << 16,
            shadow: 0,
            amb_shadow: 0,
            si_gradient: 0,
            otrap: 0,
        }
    }
}

impl SiLight {
    /// The 16 bit `Zpos` word.
    #[inline]
    pub fn zpos(&self) -> u32 {
        self.zpos_fine >> 16
    }

    #[inline]
    pub fn is_background(&self) -> bool {
        self.zpos() >= 32768
    }
}
