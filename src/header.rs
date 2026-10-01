//! The header variables a drawing states that consumers of its content need.
//!
//! A file's header holds hundreds of variables, most of them editor state (the
//! current layer, snap settings, the last view). The model carries only the
//! ones that say what the drawing's numbers mean, each as the file states it:
//! `None` is "the file does not state it", never a default filled in.

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
        let mm = HeaderVariables { insunits: Some(4) };
        assert_eq!(mm.units(), Some(Units::from_insunits(4)));
        let unitless = HeaderVariables { insunits: Some(0) };
        assert_eq!(unitless.units().map(|u| u.name), Some("du".to_string()));
    }
}
