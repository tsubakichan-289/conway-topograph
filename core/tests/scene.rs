use topograph_core::{geometry::*, scene::*, topology::Triangle};
#[test]
fn dual_sides_close_every_triangle_and_bisect_tree_edges_after_panning() {
    for anchor in [Triangle::root(), Triangle::root().neighbor(1).neighbor(0)] {
        let frame = Frame::new(anchor);
        for view in [
            Isometry::default(),
            Isometry::shift(Complex::new(0.6, -0.3)).unwrap(),
        ] {
            for max_vertices in [1, 80] {
                let scene = Scene::generate(
                    &frame,
                    view,
                    Limits {
                        max_vertices,
                        max_depth: 5,
                        ..Default::default()
                    },
                );
                let sides: std::collections::BTreeMap<_, _> = scene
                    .dual_edges
                    .iter()
                    .map(|e| (e.faces.clone(), e))
                    .collect();
                assert_eq!(
                    sides.len(),
                    scene.dual_edges.len(),
                    "shared sides are drawn once"
                );
                assert_eq!(sides.len(), 2 * scene.vertices.len() + 1);
                for vertex in &scene.vertices {
                    for i in 0..3 {
                        let mut faces = vertex.triangle.edge_faces(i);
                        faces.sort();
                        assert!(
                            sides.contains_key(&faces),
                            "even a truncated triangle has three sides"
                        );
                    }
                }
                for edge in &scene.dual_edges {
                    let a = view.apply(frame.center(&edge.from));
                    let b = view.apply(frame.center(&edge.to));
                    let m = midpoint(a, b);
                    let centered = Isometry::shift(m).unwrap();
                    let axis = centered.apply(a).unit();
                    let u = centered.apply(*edge.path.first().unwrap());
                    let v = centered.apply(*edge.path.last().unwrap());
                    assert!((distance(a, m) - distance(b, m)).abs() < 1e-7);
                    // A geodesic through the origin has antipodal ideal endpoints.
                    assert!((u + v).norm() < 1e-7, "side passes through the midpoint");
                    assert!(axis.dot(u).abs() < 1e-7, "perpendicular tangents");
                    for p in &edge.path {
                        assert!(p.finite() && p.norm() <= 1.0 + 1e-12);
                        assert!(centered.apply(*p).dot(axis).abs() < 1e-7);
                    }
                }
                for edge in &scene.edges {
                    let mut faces = edge.faces.clone();
                    faces.sort();
                    assert!(sides.contains_key(&faces));
                }
            }
        }
    }
}
#[test]
fn finite_scene_obeys_limits_and_faces_are_regions() {
    let frame = Frame::new(Triangle::root());
    for view in [
        Isometry::default(),
        Isometry::shift(Complex::new(0.95, 0.0)).unwrap(),
    ] {
        let limits = Limits {
            max_depth: 9,
            max_vertices: 180,
            min_pixels: 4.0,
            radius: 400.0,
        };
        let scene = Scene::generate(&frame, view, limits);
        assert!(scene.vertices.len() <= 180);
        assert!(
            scene
                .vertices
                .iter()
                .all(|v| v.depth <= 9 && v.center.norm() < 1.0)
        );
        assert!(
            scene
                .edges
                .iter()
                .flat_map(|e| &e.path)
                .all(|z| z.finite() && z.norm() <= 1.0 + 1e-9)
        );
        assert!(
            scene
                .patches
                .iter()
                .flat_map(|p| &p.polygon)
                .all(|z| z.finite() && z.norm() <= 1.0 + 1e-9)
        );
        for p in scene.patches.iter().filter(|p| p.clearance > 8.0) {
            assert!(polygon_contains(&p.polygon, p.label));
            assert_eq!(scene.face_at(p.label).unwrap().face, p.face);
        }
        assert!(scene.face_at(Complex::new(1.2, 0.0)).is_none());
    }
}

#[test]
fn cache_reuses_graph_and_invalidates_view_without_growing_unbounded() {
    let frame = Frame::new(Triangle::root());
    let mut cache = SceneCache::default();
    let limits = Limits {
        max_vertices: 120,
        ..Default::default()
    };
    let first = cache.update(&frame, Isometry::default(), limits).unwrap();
    assert!(cache.update(&frame, Isometry::default(), limits).is_none());
    let moved = cache
        .update(
            &frame,
            Isometry::shift(Complex::new(0.1, 0.1)).unwrap(),
            limits,
        )
        .unwrap();
    assert!(cache.hits > 0 && moved.vertices.len() <= 120 && first.vertices.len() <= 120);
    for step in 0..80 {
        let new = Frame::new(frame.triangle.neighbor(step % 3));
        cache.update(&new, Isometry::default(), limits);
        assert!(cache.stored_nodes() < 24000 + 120);
    }
}
