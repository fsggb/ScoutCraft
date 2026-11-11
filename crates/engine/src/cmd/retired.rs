//! Commands SoLeadWorks leaves out. SoLeadWorks is for people who have never touched CAD, so it
//! offers about as many functions as the camp-furniture work needs: the Scout tab, sketching, a few
//! solids, assemblies, measuring, history and views. Everything else (sheet metal, plastic parts,
//! surfaces, meshes, point clouds, motion studies, rendering, configurations, contact sets, 3D
//! sketches, surface analysis, mechanical joints, slicers…) is out of the registry: a command
//! whose id matches is neither listed, found, run nor placed on the ribbon. The code stays in the
//! tree until a later clean-up deletes it module by module.

/// Whole families, by id prefix (the dot included).
const FAMILIES: &[&str] = &[
    "render.", "surface.", "sheet.", "plastic.", "motion.", "pointcloud.", "mesh.", "canvas.", "config.", "contact.", "library.",
    "frame.", "settings.profile.", "diagnostics.crash_test",
];

/// Single commands.
const COMMANDS: &[&str] = &[
    "solid.sweep", "solid.loft", "solid.torus", "solid.coil", "solid.rib", "solid.web", "solid.emboss",
    "solid.boundary_fill", "solid.bounding_solid", "solid.draft", "solid.replace_face", "solid.scale", "solid.offset_face", "solid.split_face",
    "solid.split_body", "solid.pattern.path", "solid.shell",
    "sketch.line3d", "sketch.spline3d", "sketch.arc3d", "sketch.point3d", "sketch.move3d", "sketch.include_3d", "sketch.project_to_surface",
    "sketch.intersection_curve", "sketch.spun_profile", "sketch.isoparametric_curve", "sketch.fit_curves_to_section", "sketch.insert_svg",
    "sketch.equation_curve", "sketch.equation_curve.edit", "sketch.conic", "sketch.blend_curve",     "sketch.spline.control_point", "sketch.spline.control_point_5", "sketch.spline.insert_point", "sketch.spline.reverse",
    "inspect.curvature_comb", "inspect.minimum_radius", "inspect.isocurve", "inspect.zebra", "inspect.environment_map",
    "inspect.accessibility", "inspect.curvature_map", "inspect.continuity", "inspect.draft", "inspect.parameter_sweep",
    "file.insert_mesh", "file.save_mesh", "file.send_to_slicer", "file.open_in_easyprint", "file.slicers", "body.print_face",
    "joint.gear", "joint.rack_pinion", "joint.screw", "joint.cam", "joint.reach", "joint.motion_link", "joint.drive", "joint.sweep",
    "view.render_focus", "view.render_frame", "view.render_browser", "view.render_inspector", "view.render_select", "view.render_stage",
    "view.shots_strip", "view.render_dock", "view.ambient_occlusion",
    "parameters.users", "material.properties.validate", "material.properties.directional",
];

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
            "assembly.bom", "timeline.rollback", "edit.checkpoint", "edit.restore_checkpoint", "inspect.interference",
            "inspect.measure", "parameters.change",
        ] {
            assert!(!is_retired(id), "{id} must stay");
        }
    }

    #[test]
    fn sheet_metal_and_rendering_are_gone() {
        for id in ["sheet.flange", "render.start", "surface.loft", "plastic.rib", "motion.study", "solid.sweep", "joint.gear", "mesh.convert"] {
            assert!(is_retired(id), "{id} must be retired");
        }
    }
}
