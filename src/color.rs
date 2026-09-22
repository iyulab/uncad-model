//! The AutoCAD Color Index (ACI) palette: the one piece of color data that
//! belongs to the format rather than to any consumer.
//!
//! Only the table and its lookup live here. Resolving an entity's effective
//! color (BYLAYER/BYBLOCK precedence, true-color override) and choosing how
//! to show it (a hex string, a background-dependent flip of white to black)
//! are a renderer's decisions and are not part of the model.

/// The AutoCAD Color Index palette: packed 24-bit RGB (`0xRRGGBB`) for
/// indices 0 through 256. These are the historical color choices of the
/// format, not derivable from a formula; index 0 (BYBLOCK) and index 256
/// (BYLAYER) are placeholders that hold `0`, since neither is a color of its
/// own.
#[rustfmt::skip]
pub const ACI_PALETTE: [u32; 257] = [
    0, 16711680, 16776960, 65280, 65535, 255, 16711935, 16777215, 8421504,
    12632256, 16711680, 16744319, 13369344, 13395558, 10027008, 10046540, 8323072,
    8339263, 4980736, 4990502, 16727808, 16752511, 13382400, 13401958, 10036736,
    10051404, 8331008, 8343359, 4985600, 4992806, 16744192, 16760703, 13395456,
    13408614, 10046464, 10056268, 8339200, 8347455, 4990464, 4995366, 16760576,
    16768895, 13408512, 13415014, 10056192, 10061132, 8347392, 8351551, 4995328,
    4997670, 16776960, 16777087, 13421568, 13421670, 10000384, 10000460, 8355584,
    8355647, 5000192, 5000230, 12582656, 14679935, 10079232, 11717734, 7510016,
    8755276, 6258432, 7307071, 3755008, 4344870, 8388352, 12582783, 6736896,
    10079334, 5019648, 7510092, 4161280, 6258495, 2509824, 3755046, 4194048,
    10485631, 3394560, 8375398, 2529280, 6264908, 2064128, 5209919, 1264640,
    3099686, 65280, 8388479, 52224, 6736998, 38912, 5019724, 32512, 4161343,
    19456, 2509862, 65343, 8388511, 52275, 6737023, 38950, 5019743, 32543,
    4161359, 19475, 2509871, 65407, 8388543, 52326, 6737049, 38988, 5019762,
    32575, 4161375, 19494, 2509881, 65471, 8388575, 52377, 6737074, 39026,
    5019781, 32607, 4161391, 19513, 2509890, 65535, 8388607, 52428, 6737100,
    39064, 5019800, 32639, 4161407, 19532, 2509900, 49151, 8380415, 39372,
    6730444, 29336, 5014936, 24447, 4157311, 14668, 2507340, 32767, 8372223,
    26316, 6724044, 19608, 5010072, 16255, 4153215, 9804, 2505036, 16383, 8364031,
    13260, 6717388, 9880, 5005208, 8063, 4149119, 4940, 2502476, 255, 8355839,
    204, 6710988, 152, 5000344, 127, 4145023, 76, 2500172, 4129023, 10452991,
    3342540, 8349388, 2490520, 6245528, 2031743, 5193599, 1245260, 3089996,
    8323327, 12550143, 6684876, 10053324, 4980888, 7490712, 4128895, 6242175,
    2490444, 3745356, 12517631, 14647295, 10027212, 11691724, 7471256, 8735896,
    6226047, 7290751, 3735628, 4335180, 16711935, 16744447, 13369548, 13395660,
    9961624, 9981080, 8323199, 8339327, 4980812, 4990540, 16711871, 16744415,
    13369497, 13395634, 9961586, 9981061, 8323167, 8339311, 4980793, 4990530,
    16711807, 16744383, 13369446, 13395609, 9961548, 9981042, 8323135, 8339295,
    4980774, 4990521, 16711743, 16744351, 13369395, 13395583, 9961510, 9981023,
    8323103, 8339279, 4980755, 4990511, 3355443, 5987163, 8684676, 11382189,
    14079702, 16777215, 0,
];

/// The packed RGB of an ACI index, or `None` when `index` is outside the
/// table. Total over `0..=256`; the two placeholder slots (0 and 256) yield
/// `Some(0)`, and it is the caller's job to treat them as BYBLOCK/BYLAYER
/// rather than as black.
pub fn aci_to_rgb(index: u16) -> Option<u32> {
    ACI_PALETTE.get(usize::from(index)).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aci_index_2_is_yellow() {
        assert_eq!(aci_to_rgb(2), Some(0xFF_FF00));
    }

    /// Pins the palette to the long-published ACI table rather than to itself.
    ///
    /// There is no single canonical ACI-to-RGB mapping: AutoCAD's *displayed*
    /// colours depend on the drawing-area background, so a table captured from
    /// a dark model space and one captured from a white sheet disagree. What
    /// this crate carries is the table that has been published unchanged for
    /// decades, which is also the one a reader can check against any of the
    /// widely mirrored ACI charts.
    ///
    /// The samples below are the entries those charts agree on and that are
    /// easiest to verify by eye: the first primary, the two greys that sit
    /// right where implementations tend to diverge (8 and 9), the head of the
    /// red ramp, and the grey ladder at the end. A silent edit to
    /// `ACI_PALETTE` -- a re-import from some other project's table, say --
    /// changes at least one of these.
    #[test]
    fn aci_palette_matches_the_published_table() {
        for (index, expected) in [
            (1_usize, 0xFF_0000_u32),
            (8, 0x80_8080),
            (9, 0xC0_C0C0),
            (11, 0xFF_7F7F),
            (12, 0xCC_0000),
            (13, 0xCC_6666),
            (14, 0x99_0000),
            (15, 0x99_4C4C),
            (16, 0x7F_0000),
            (250, 0x33_3333),
            (251, 0x5B_5B5B),
            (252, 0x84_8484),
            (253, 0xAD_ADAD),
            (254, 0xD6_D6D6),
            (255, 0xFF_FFFF),
        ] {
            assert_eq!(
                ACI_PALETTE[index], expected,
                "ACI {index} is #{:06x}, expected #{expected:06x}",
                ACI_PALETTE[index]
            );
        }
    }

    /// Index 0 is a placeholder and 256 is the BYLAYER slot; neither is a
    /// colour anyone should read out of the table, and both being zero is what
    /// keeps `aci_to_rgb` total over `0..=256` without a branch.
    #[test]
    fn aci_palette_covers_zero_through_byblock_and_bylayer() {
        assert_eq!(ACI_PALETTE.len(), 257);
        assert_eq!(ACI_PALETTE[0], 0);
        assert_eq!(ACI_PALETTE[256], 0);
        assert_eq!(aci_to_rgb(256), Some(0));
        assert_eq!(aci_to_rgb(257), None);
    }
}
