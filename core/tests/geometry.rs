use topograph_core::{geometry::*, topology::Triangle};
fn close(a: Complex, b: Complex) {
    assert!((a - b).norm() < 1e-9, "{a:?} != {b:?}");
}
#[test]
fn cayley_roundtrip() {
    for z in [Complex::I, Complex::new(1.0, 2.0), Complex::new(-7.0, 0.1)] {
        assert!(cayley(z).norm() < 1.0);
        close(inverse_cayley(cayley(z)), z);
    }
}
#[test]
fn isometry_preserves_disk_distance_and_drag_anchor() {
    let t = Isometry::shift(Complex::new(0.7, -0.3)).unwrap();
    for i in 0..200 {
        let angle = i as f64 * 0.31;
        let z = Complex::new(angle.cos(), angle.sin()) * 0.99;
        assert!(t.apply(z).norm() < 1.0);
        close(t.inverse().apply(t.apply(z)), z);
    }
    let a = Complex::new(-0.2, 0.3);
    let b = Complex::new(0.4, -0.5);
    assert!((distance(a, b) - distance(t.apply(a), t.apply(b))).abs() < 1e-10);
    close(t.drag(a, b).apply(t.inverse().apply(a)), b);
    let mut product = Isometry::default();
    for _ in 0..10000 {
        product = t.inverse().compose(t).compose(product);
    }
    close(product.apply(a), a);
    assert!(product.valid());
}
#[test]
fn farey_incenters_and_frame_changes() {
    let root = Triangle::root();
    let frame = Frame::new(root.clone());
    close(frame.center(&root), Complex::ZERO);
    for t in root.neighbors() {
        let center = frame.center(&t);
        assert!((distance(Complex::ZERO, center) - 3f64.ln()).abs() < 1e-9);
        let new = Frame::new(t.clone());
        let map = frame.transition(&new);
        close(map.apply(new.center(&t)), center);
        for v in t.faces() {
            close(map.apply(new.ideal(v)), frame.ideal(v));
        }
    }
    // Ideal-side circles: incenter has equal hyperbolic distance to all three sides.
    for t in root.neighbors() {
        let center = frame.center(&t);
        let shift = Isometry::shift(center).unwrap();
        let mut values = Vec::new();
        for i in 0..3 {
            let [u, v] = t.edge_faces(i);
            let a = shift.apply(frame.ideal(&u));
            let b = shift.apply(frame.ideal(&v));
            let angle = (a.dot(b)).clamp(-1.0, 1.0).acos() / 2.0;
            values.push((1.0 - angle.sin()) / angle.cos());
        }
        assert!((values[0] - values[1]).abs() < 1e-9 && (values[1] - values[2]).abs() < 1e-9);
    }
}
#[test]
fn geodesics_are_orthogonal_circles_or_diameters() {
    let a = Complex::new(0.2, 0.3);
    let b = Complex::new(-0.5, 0.1);
    let det = a.cross(b);
    let ka = (1.0 + a.norm_sq()) / 2.0;
    let kb = (1.0 + b.norm_sq()) / 2.0;
    let c = Complex::new((ka * b.im - kb * a.im) / det, (a.re * kb - b.re * ka) / det);
    let r2 = c.norm_sq() - 1.0;
    let path = geodesic(a, b, 80);
    close(path[0], a);
    close(*path.last().unwrap(), b);
    for p in path {
        assert!(p.norm() < 1.0);
        assert!(((p - c).norm_sq() - r2).abs() < 1e-9);
    }
    for p in geodesic(Complex::new(-0.4, 0.0), Complex::new(0.7, 0.0), 20) {
        assert!(p.im.abs() < 1e-14);
    }
}
