//! Built-in gradients for fountain fills: named sets of stops the fill
//! editor offers as swatches (a click replaces the stops, keeping the
//! type and angle). Original colour sets, see
//! docs/behavior/gradient-fill.md.

use tracedraw_core::{Color, Stop};

/// The i18n key of each gradient's name and its stops (position 0 to 1,
/// RGB).
pub const PRESETS: [(&str, &[(f64, u32)]); 24] = [
    ("grad.black_white", &[(0.0, 0x000000), (1.0, 0xFFFFFF)]),
    (
        "grad.sunset",
        &[
            (0.0, 0x2B1055),
            (0.45, 0xD7385E),
            (0.75, 0xF58A4B),
            (1.0, 0xFDD36A),
        ],
    ),
    (
        "grad.ocean",
        &[(0.0, 0x03256C), (0.5, 0x1768AC), (1.0, 0x06BEE1)],
    ),
    (
        "grad.rainbow",
        &[
            (0.0, 0xE53935),
            (0.2, 0xFB8C00),
            (0.4, 0xFDD835),
            (0.6, 0x43A047),
            (0.8, 0x1E88E5),
            (1.0, 0x8E24AA),
        ],
    ),
    (
        "grad.gold",
        &[
            (0.0, 0x7A5410),
            (0.3, 0xE6C15C),
            (0.5, 0xFFF1B8),
            (0.7, 0xD9A934),
            (1.0, 0x7A5410),
        ],
    ),
    (
        "grad.silver",
        &[
            (0.0, 0x5E6670),
            (0.3, 0xC9CED4),
            (0.5, 0xFFFFFF),
            (0.7, 0xB4BAC1),
            (1.0, 0x5E6670),
        ],
    ),
    (
        "grad.copper",
        &[
            (0.0, 0x5C2A12),
            (0.35, 0xC8743E),
            (0.55, 0xF2B38A),
            (0.75, 0xB3612F),
            (1.0, 0x5C2A12),
        ],
    ),
    (
        "grad.chrome",
        &[
            (0.0, 0xFFFFFF),
            (0.45, 0x8A9AA9),
            (0.5, 0x2E3A44),
            (0.55, 0xC4A87A),
            (1.0, 0xFFFFFF),
        ],
    ),
    (
        "grad.fire",
        &[
            (0.0, 0x3B0000),
            (0.35, 0xC21807),
            (0.65, 0xFF7B00),
            (1.0, 0xFFE45E),
        ],
    ),
    (
        "grad.ice",
        &[(0.0, 0xFFFFFF), (0.5, 0xBDE7F5), (1.0, 0x4FA3D1)],
    ),
    (
        "grad.forest",
        &[(0.0, 0x0B3D20), (0.5, 0x2E7D32), (1.0, 0xA5D46A)],
    ),
    (
        "grad.sky",
        &[(0.0, 0x1E5AA8), (0.6, 0x7FB8E8), (1.0, 0xE9F4FF)],
    ),
    (
        "grad.dusk",
        &[(0.0, 0x20214F), (0.5, 0x8A6FDF), (1.0, 0xE8505B)],
    ),
    (
        "grad.aurora",
        &[
            (0.0, 0x061A40),
            (0.4, 0x0B8F6A),
            (0.7, 0x5EE6A8),
            (1.0, 0x9B5DE5),
        ],
    ),
    (
        "grad.pastel",
        &[(0.0, 0xF8BBD0), (0.5, 0xD1C4E9), (1.0, 0xB3E5FC)],
    ),
    (
        "grad.neon",
        &[(0.0, 0xFF00E5), (0.5, 0x7C4DFF), (1.0, 0x00E5FF)],
    ),
    ("grad.peach", &[(0.0, 0xFFD8B8), (1.0, 0xFF8A65)]),
    ("grad.mint", &[(0.0, 0xE0F7EF), (1.0, 0x26A69A)]),
    (
        "grad.berry",
        &[(0.0, 0x4A0E4E), (0.5, 0xA4245C), (1.0, 0xF06292)],
    ),
    ("grad.lavender", &[(0.0, 0xF3E5F5), (1.0, 0x7E57C2)]),
    (
        "grad.desert",
        &[(0.0, 0x8D5524), (0.5, 0xD9A066), (1.0, 0xF6E2B3)],
    ),
    (
        "grad.steel",
        &[(0.0, 0x263238), (0.5, 0x607D8B), (1.0, 0xCFD8DC)],
    ),
    (
        "grad.sepia",
        &[(0.0, 0x2E1B0E), (0.5, 0x8B6A47), (1.0, 0xF1E3C8)],
    ),
    ("grad.charcoal", &[(0.0, 0x111111), (1.0, 0x6D6D6D)]),
];

/// The stops of preset `i`.
pub fn stops(i: usize) -> Option<Vec<Stop>> {
    let (_, list) = PRESETS.get(i)?;
    Some(
        list.iter()
            .map(|(pos, c)| Stop {
                pos: *pos,
                color: Color::rgb8((c >> 16) as u8, (c >> 8) as u8, *c as u8),
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_preset_runs_from_0_to_1_in_order() {
        for i in 0..PRESETS.len() {
            let s = stops(i).expect("stops");
            assert!(s.len() >= 2, "{}", PRESETS[i].0);
            assert_eq!(s[0].pos, 0.0);
            assert_eq!(s[s.len() - 1].pos, 1.0);
            assert!(
                s.windows(2).all(|w| w[0].pos < w[1].pos),
                "{}",
                PRESETS[i].0
            );
        }
        assert!(stops(PRESETS.len()).is_none());
    }
}
