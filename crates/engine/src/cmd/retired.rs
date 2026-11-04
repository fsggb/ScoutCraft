//! Commands SoLeadWorks leaves out. SoLeadWorks keeps SolidCraft whole (its layout, its ribbon, its
//! history, assemblies, surfaces, motion and rendering can all serve a camp project); only what a
//! scout workshop has no use for goes: sheet metal, plastic parts, meshes, point clouds and the
//! 3D-printing hand-offs. A command whose id matches is neither listed, found, run nor placed on
//! the ribbon. The code stays in the tree until a later clean-up deletes it module by module.

/// Whole families, by id prefix (the dot included).
const FAMILIES: &[&str] = &["sheet.", "plastic.", "pointcloud.", "mesh.", "diagnostics.crash_test"];

/// Single commands: 3D-printing hand-offs, which a camp workshop has no use for.
const COMMANDS: &[&str] = &["file.insert_mesh", "file.save_mesh", "file.send_to_slicer", "file.open_in_easyprint", "file.slicers", "body.print_face"];

/// Whether SoLeadWorks leaves this command out.
pub fn is_retired(id: &str) -> bool {
    COMMANDS.contains(&id) || FAMILIES.iter().any(|f| id.starts_with(f))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_woodworking_core_is_kept() {
        for id in [
            "solid.extrude", "solid.box", "solid.cylinder", "solid.hole", "solid.combine", "solid.move", "solid.mirror", "sketch.create",
            "sketch.rectangle.two_point", "sketch.dimension", "component.create", "joint.create", "occurrence.move", "explode.create",
            "assembly.bom", "frame.member", "timeline.rollback", "edit.checkpoint", "edit.restore_checkpoint", "inspect.interference", "render.start", "surface.loft", "motion.study", "solid.sweep",
            "inspect.measure", "parameters.change",
        ] {
            assert!(!is_retired(id), "{id} must stay");
        }
    }

    #[test]
    fn sheet_metal_and_rendering_are_gone() {
        for id in ["sheet.flange", "plastic.rib", "mesh.convert", "file.send_to_slicer"] {
            assert!(is_retired(id), "{id} must be retired");
        }
    }
}
