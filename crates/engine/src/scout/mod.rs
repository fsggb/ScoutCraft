//! SoLeadWorks' scout layer, in the order it is built: `pole` (round poles and boards with the
//! operations of froissartage: tenon and mortise, half-lap, peg, lashing, split, point), `models`
//! (the eleven furniture of CathIA, rebuilt from poles) and `plan` (the plans and the HTML page
//! drawn from them). Pure data in, data out; the commands that build them in a design are
//! `cmd::scout`.

pub mod dessin;
pub mod models;
pub mod nomenclature;
pub mod plan;
pub mod pole;
