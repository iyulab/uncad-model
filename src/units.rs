//! The drawing unit a `$INSUNITS` code names.
//!
//! The DXF reference defines the codes and what each one measures; a
//! drawing's header states the code. The table lives here once so that every
//! consumer giving a drawing's numbers a unit reads the same one.

use serde::{Deserialize, Serialize};

/// The drawing unit a `$INSUNITS` code names, with its conversion to
/// millimetres when the code is a length.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Units {
    /// Short name: `"mm"`, `"in"`, `"ft"`, `"m"`, ... -- or `"du"` ("drawing
    /// units") for code 0 (unitless) and for a code the DXF reference does
    /// not define.
    pub name: String,
    /// Millimetres per drawing unit; `None` when unitless or unknown.
    pub to_mm: Option<f64>,
}

impl Units {
    /// The DXF reference's `$INSUNITS` table (codes 0..=24).
    pub fn from_insunits(code: u16) -> Units {
        let (name, to_mm): (&str, Option<f64>) = match code {
            1 => ("in", Some(25.4)),
            2 => ("ft", Some(304.8)),
            3 => ("mi", Some(1_609_344.0)),
            4 => ("mm", Some(1.0)),
            5 => ("cm", Some(10.0)),
            6 => ("m", Some(1000.0)),
            7 => ("km", Some(1_000_000.0)),
            8 => ("uin", Some(2.54e-5)),
            9 => ("mil", Some(0.0254)),
            10 => ("yd", Some(914.4)),
            11 => ("angstrom", Some(1e-7)),
            12 => ("nm", Some(1e-6)),
            13 => ("um", Some(1e-3)),
            14 => ("dm", Some(100.0)),
            15 => ("dam", Some(10_000.0)),
            16 => ("hm", Some(100_000.0)),
            17 => ("Gm", Some(1e12)),
            18 => ("au", Some(1.495_978_707e14)),
            19 => ("ly", Some(9.460_730_472_580_8e18)),
            20 => ("pc", Some(3.085_677_581_491_367e19)),
            // US survey units: 1200/3937 m to the foot.
            21 => ("us-ft", Some(304.800_609_601_219_2)),
            22 => ("us-in", Some(25.400_050_800_101_6)),
            23 => ("us-yd", Some(914.401_828_803_657_7)),
            24 => ("us-mi", Some(1_609_347.218_694_437_2)),
            _ => ("du", None),
        };
        Units {
            name: name.to_string(),
            to_mm,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_common_codes_name_their_unit() {
        assert_eq!(Units::from_insunits(4).name, "mm");
        assert_eq!(Units::from_insunits(4).to_mm, Some(1.0));
        assert_eq!(Units::from_insunits(1).to_mm, Some(25.4));
        assert_eq!(Units::from_insunits(6).to_mm, Some(1000.0));
    }

    #[test]
    fn unitless_and_undefined_codes_have_no_length() {
        for code in [0, 25, 999] {
            let u = Units::from_insunits(code);
            assert_eq!((u.name.as_str(), u.to_mm), ("du", None), "{code}");
        }
    }

    #[test]
    fn us_survey_units_are_1200_over_3937_metres_to_the_foot() {
        let foot = 1_200_000.0 / 3937.0;
        let close = |a: f64, b: f64| (a - b).abs() <= 1e-12 * b;
        assert!(close(Units::from_insunits(21).to_mm.unwrap(), foot));
        assert!(close(Units::from_insunits(22).to_mm.unwrap(), foot / 12.0));
        assert!(close(Units::from_insunits(23).to_mm.unwrap(), foot * 3.0));
        assert!(close(
            Units::from_insunits(24).to_mm.unwrap(),
            foot * 5280.0
        ));
    }
}
