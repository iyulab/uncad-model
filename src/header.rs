//! The header variables a drawing states that consumers of its content need.
//!
//! A file's header holds hundreds of variables, most of them editor state (the
//! current layer, snap settings, the last view). The model carries only the
//! ones that say what the drawing's numbers mean, the ones that say how an
//! entity is shown where its own record does not (how a POINT is drawn), and
//! the identifiers the file states for where the drawing came from and for
//! this save, each as the file states it: `None` is "the file does not state
//! it", never a default filled in.

use serde::{Deserialize, Serialize};

use crate::units::Units;

/// The header variables the model carries.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct HeaderVariables {
    /// `$INSUNITS` (DXF group 70): the code naming the drawing's unit, as the
    /// file states it. `None` when the file does not state one -- a DXF
    /// without the variable, or a DWG of a version before the variable
    /// existed (R2000). A stated code the DXF reference does not define is
    /// kept as stated; [`units`](Self::units) names it `"du"`.
    pub insunits: Option<u16>,
    /// `$FINGERPRINTGUID` (DXF group 2): the identifier the drawing was given
    /// when it was created, kept through every later save -- including by a
    /// drawing created as a copy of another, or from the same template, so
    /// two unrelated drawings can state the same value. It says where a
    /// drawing came from, not which drawing it is. Kept as stated (braces
    /// included). `None` when the file does not state it: a DXF without the
    /// variable, or a DWG before R2000.
    #[serde(default)]
    pub fingerprintguid: Option<String>,
    /// `$VERSIONGUID` (DXF group 2): the identifier of the drawing's state as
    /// of a save, given anew when a save changes it. Kept as stated; `None`
    /// as for [`fingerprintguid`](Self::fingerprintguid).
    #[serde(default)]
    pub versionguid: Option<String>,
    /// `$PDMODE` (DXF group 70): how every POINT of the drawing is shown --
    /// [`point_display`](Self::point_display) reads it. Kept as stated;
    /// `None` when the file does not state it (and for a document written
    /// before this field existed).
    #[serde(default)]
    pub pdmode: Option<i16>,
    /// `$PDSIZE` (DXF group 40): the size of the figure a POINT is shown
    /// with -- [`point_display`](Self::point_display) reads it. Kept as
    /// stated; `None` as for [`pdmode`](Self::pdmode).
    #[serde(default)]
    pub pdsize: Option<f64>,
}

/// How a POINT is shown, as `$PDMODE` and `$PDSIZE` define it
/// ([`HeaderVariables::point_display`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointDisplay {
    /// The figure drawn through the point (`$PDMODE`'s low part, 0 to 4).
    pub figure: PointFigure,
    /// A circle around the point (`$PDMODE` 32 added).
    pub circle: bool,
    /// A square around the point (`$PDMODE` 64 added).
    pub square: bool,
    /// The size of the figure, the circle and the square: the circle's
    /// diameter, the square's side, the length of the plus's and the
    /// cross's strokes. `None` when the file does not state `$PDSIZE` or
    /// states a value that is not finite. The dot does not use it.
    pub size: Option<PointSize>,
}

/// The figure a POINT is drawn with (`$PDMODE` 0 to 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointFigure {
    /// 0: a dot -- the smallest mark the display draws, whatever the size.
    Dot,
    /// 1: nothing.
    Nothing,
    /// 2: a plus, its strokes along the axes.
    Plus,
    /// 3: a cross, its strokes along the diagonals.
    Cross,
    /// 4: a short stroke up from the point, half the size long.
    Tick,
}

/// How big a POINT's figure is (`$PDSIZE`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PointSize {
    /// A value above 0: that many drawing units.
    Absolute(f64),
    /// 0 or below: a fraction of the height of the view the drawing is shown
    /// in -- 0 is 5 %, a negative value its magnitude in percent. The figure
    /// keeps its size on screen however far the view is zoomed.
    ViewFraction(f64),
}

impl HeaderVariables {
    /// How a POINT is shown, read from `$PDMODE` and `$PDSIZE`. `None` when
    /// the file does not state `$PDMODE`, or states a value the format does
    /// not define (a figure above 4, or bits other than 32 and 64 added).
    pub fn point_display(&self) -> Option<PointDisplay> {
        let mode = self.pdmode?;
        const CIRCLE: i16 = 32;
        const SQUARE: i16 = 64;
        if mode < 0 || mode & !(CIRCLE | SQUARE | 0x1F) != 0 {
            return None;
        }
        let figure = match mode & 0x1F {
            0 => PointFigure::Dot,
            1 => PointFigure::Nothing,
            2 => PointFigure::Plus,
            3 => PointFigure::Cross,
            4 => PointFigure::Tick,
            _ => return None,
        };
        let size = self.pdsize.filter(|s| s.is_finite()).map(|s| match s {
            s if s > 0.0 => PointSize::Absolute(s),
            // 0 is 5 %; below it, the magnitude in percent.
            s if s < 0.0 => PointSize::ViewFraction(-s / 100.0),
            _ => PointSize::ViewFraction(0.05),
        });
        Some(PointDisplay {
            figure,
            circle: mode & CIRCLE != 0,
            square: mode & SQUARE != 0,
            size,
        })
    }
}

impl HeaderVariables {
    /// The unit `$INSUNITS` names, when the file states the variable.
    pub fn units(&self) -> Option<Units> {
        self.insunits.map(Units::from_insunits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units_follow_the_stated_code_only() {
        assert_eq!(HeaderVariables::default().units(), None);
        let mm = HeaderVariables {
            insunits: Some(4),
            ..HeaderVariables::default()
        };
        assert_eq!(mm.units(), Some(Units::from_insunits(4)));
        let unitless = HeaderVariables {
            insunits: Some(0),
            ..HeaderVariables::default()
        };
        assert_eq!(unitless.units().map(|u| u.name), Some("du".to_string()));
    }

    fn display(pdmode: Option<i16>, pdsize: Option<f64>) -> Option<PointDisplay> {
        HeaderVariables {
            pdmode,
            pdsize,
            ..HeaderVariables::default()
        }
        .point_display()
    }

    #[test]
    fn a_point_is_shown_as_pdmode_and_pdsize_say() {
        assert_eq!(display(None, Some(1.0)), None);
        let dot = display(Some(0), Some(0.0)).unwrap();
        assert_eq!(dot.figure, PointFigure::Dot);
        assert!(!dot.circle && !dot.square);
        assert_eq!(dot.size, Some(PointSize::ViewFraction(0.05)));
        let circled_cross = display(Some(35), Some(2.5)).unwrap();
        assert_eq!(circled_cross.figure, PointFigure::Cross);
        assert!(circled_cross.circle && !circled_cross.square);
        assert_eq!(circled_cross.size, Some(PointSize::Absolute(2.5)));
        let boxed = display(Some(98), Some(-10.0)).unwrap();
        assert_eq!(boxed.figure, PointFigure::Plus);
        assert!(boxed.circle && boxed.square);
        assert_eq!(boxed.size, Some(PointSize::ViewFraction(0.1)));
        assert_eq!(display(Some(1), None).unwrap().figure, PointFigure::Nothing);
        assert_eq!(display(Some(4), None).unwrap().size, None);
    }

    #[test]
    fn a_pdmode_the_format_does_not_define_is_not_read() {
        for mode in [5, 31, 16, 128, 36 + 128, -1] {
            assert_eq!(display(Some(mode), Some(1.0)), None, "{mode}");
        }
        assert_eq!(display(Some(2), Some(f64::NAN)).unwrap().size, None);
    }
}
