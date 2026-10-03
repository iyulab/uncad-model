//! The header variables a drawing states that consumers of its content need.
//!
//! A file's header holds hundreds of variables, most of them editor state (the
//! current layer, snap settings, the last view). The model carries only the
//! ones that say what the drawing's numbers mean, and the identifiers the file
//! states for where the drawing came from and for this save, each as the file
//! states it: `None` is "the file does not state it", never a default filled
//! in.

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
}
