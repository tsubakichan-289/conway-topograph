use crate::{
    algebra::Primitive,
    geometry::{
        Complex, Frame, Isometry, geodesic, ideal_geodesic, midpoint, polygon_contains, ray,
    },
    topology::Triangle,
};
use std::collections::{BTreeSet, HashMap, VecDeque};

#[derive(Default)]
pub struct SceneCache {
    neighbors: HashMap<Triangle, [Triangle; 3]>,
    centers: HashMap<Triangle, Complex>,
    ideals: HashMap<Primitive, Complex>,
    anchor: Option<Triangle>,
    last: Option<(Triangle, Isometry, Limits)>,
    pub hits: usize,
    pub misses: usize,
}
impl SceneCache {
    /// None means the previous scene is still valid; no allocation or generation occurs.
    pub fn update(&mut self, frame: &Frame, view: Isometry, limits: Limits) -> Option<Scene> {
        let key = (frame.triangle.clone(), view, limits);
        if self.last.as_ref() == Some(&key) {
            return None;
        }
        self.prepare(frame);
        let scene = Scene::generate_cached(frame, view, limits, self);
        self.last = Some(key);
        Some(scene)
    }
    fn prepare(&mut self, frame: &Frame) {
        if self.anchor.as_ref() != Some(&frame.triangle) || self.centers.len() > 24000 {
            self.centers.clear();
            self.ideals.clear();
            self.anchor = Some(frame.triangle.clone());
        }
        if self.neighbors.len() > 24000 {
            self.neighbors.clear();
        }
    }
    fn neighbors(&mut self, t: &Triangle) -> [Triangle; 3] {
        if let Some(n) = self.neighbors.get(t) {
            self.hits += 1;
            n.clone()
        } else {
            self.misses += 1;
            let n = t.neighbors();
            self.neighbors.insert(t.clone(), n.clone());
            n
        }
    }
    fn center(&mut self, frame: &Frame, t: &Triangle) -> Complex {
        *self
            .centers
            .entry(t.clone())
            .or_insert_with(|| frame.center(t))
    }
    fn ideal(&mut self, frame: &Frame, v: &Primitive) -> Complex {
        *self
            .ideals
            .entry(v.clone())
            .or_insert_with(|| frame.ideal(v))
    }
    pub fn stored_nodes(&self) -> usize {
        self.neighbors.len()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Limits {
    pub max_depth: usize,
    pub max_vertices: usize,
    pub min_pixels: f64,
    pub radius: f64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_depth: 24,
            max_vertices: 2500,
            min_pixels: 5.0,
            radius: 400.0,
        }
    }
}
#[derive(Clone, Debug)]
pub struct Vertex {
    pub triangle: Triangle,
    pub center: Complex,
    pub depth: usize,
}
#[derive(Clone, Debug)]
pub struct Edge {
    pub from: Triangle,
    pub to: Triangle,
    pub faces: [Primitive; 2],
    pub path: Vec<Complex>,
}
#[derive(Clone, Debug)]
pub struct Patch {
    pub face: Primitive,
    pub triangle: Triangle,
    pub polygon: Vec<Complex>,
    pub triangles: Vec<[usize; 3]>,
    pub label: Complex,
    pub clearance: f64,
}
#[derive(Clone, Debug, Default)]
pub struct Scene {
    pub vertices: Vec<Vertex>,
    pub edges: Vec<Edge>,
    /// Farey sides, including the boundary of the finite collection of triangles.
    pub dual_edges: Vec<Edge>,
    pub patches: Vec<Patch>,
    pub truncated: bool,
    pub pruned: usize,
}
impl Scene {
    pub fn generate(frame: &Frame, view: Isometry, limits: Limits) -> Self {
        let mut cache = SceneCache::default();
        cache.prepare(frame);
        Self::generate_cached(frame, view, limits, &mut cache)
    }
    fn generate_cached(
        frame: &Frame,
        view: Isometry,
        limits: Limits,
        cache: &mut SceneCache,
    ) -> Self {
        let mut scene = Self::default();
        let mut queue = VecDeque::from([(frame.triangle.clone(), None, 0)]);
        let mut seen = BTreeSet::new();
        let mut dual_seen = BTreeSet::new();
        while let Some((triangle, parent, depth)) = queue.pop_front() {
            if scene.vertices.len() >= limits.max_vertices.min(12000) {
                scene.truncated = true;
                break;
            }
            if !seen.insert(triangle.clone()) {
                continue;
            }
            let center = view.apply(cache.center(frame, &triangle));
            if !center.finite() || center.norm() >= 1.0 - 1e-12 {
                scene.pruned += 1;
                continue;
            }
            let neighbors = cache.neighbors(&triangle);
            let centers = neighbors
                .clone()
                .map(|t| view.apply(cache.center(frame, &t)).bounded(1.0 - 1e-12));
            for i in 0..3 {
                let next = &neighbors[i];
                let [u, v] = triangle.edge_faces(i);
                let a = view.apply(cache.ideal(frame, &u)).unit();
                let b = view.apply(cache.ideal(frame, &v)).unit();
                let mut faces = [u.clone(), v.clone()];
                faces.sort();
                if dual_seen.insert(faces.clone()) {
                    scene.dual_edges.push(Edge {
                        from: triangle.clone(),
                        to: next.clone(),
                        faces,
                        path: ideal_geodesic(a, b, 32),
                    });
                }
                if Some(next) == parent.as_ref() {
                    continue;
                }
                // In Klein coordinates the Farey side is the chord (a,b).
                // Prune the whole opposite subtree only when its ideal cap is small.
                let klein = center * 2.0 / (1.0 + center.norm_sq());
                let parent_side = (b - a).cross(klein - a);
                let origin_side = (b - a).cross(-a);
                let tiny = parent_side * origin_side >= 0.0
                    && (a - b).norm() * limits.radius < limits.min_pixels;
                let small = (center - centers[i]).norm() * limits.radius < 0.15;
                if depth >= limits.max_depth.min(96) || tiny || small {
                    scene.pruned += 1;
                    continue;
                }
                scene.edges.push(Edge {
                    from: triangle.clone(),
                    to: next.clone(),
                    faces: [u, v],
                    path: geodesic(center, centers[i], 24),
                });
                queue.push_back((next.clone(), Some(triangle.clone()), depth + 1));
            }
            for j in 0..3 {
                let a = midpoint(center, centers[(j + 1) % 3]);
                let b = midpoint(center, centers[(j + 2) % 3]);
                let ideal = view.apply(cache.ideal(frame, &triangle.faces()[j])).unit();
                let mut polygon = geodesic(center, a, 6);
                polygon.extend(ray(a, ideal, 14).into_iter().skip(1));
                polygon.extend(ray(b, ideal, 14).into_iter().rev().skip(1));
                polygon.extend(geodesic(b, center, 6).into_iter().skip(1));
                polygon.pop();
                // Label lies inside this sector, toward the cusp, away from the dual vertex.
                let label = geodesic(center, ideal.bounded(0.99999999), 8)[3];
                let clearance = polygon
                    .iter()
                    .zip(polygon.iter().cycle().skip(1))
                    .take(polygon.len())
                    .map(|(&a, &b)| segment_distance(label, a, b))
                    .fold(f64::INFINITY, f64::min)
                    * limits.radius;
                scene.patches.push(Patch {
                    face: triangle.faces()[j].clone(),
                    triangle: triangle.clone(),
                    triangles: triangulate(&polygon),
                    polygon,
                    label,
                    clearance,
                });
            }
            scene.vertices.push(Vertex {
                triangle,
                center,
                depth,
            });
        }
        // Only retain edges whose endpoints made it into this finite scene.
        let present: BTreeSet<_> = scene.vertices.iter().map(|v| v.triangle.clone()).collect();
        scene.edges.retain(|e| present.contains(&e.to));
        scene
    }
    pub fn face_at(&self, point: Complex) -> Option<&Patch> {
        if point.norm() >= 1.0 {
            return None;
        }
        self.patches
            .iter()
            .find(|p| polygon_contains(&p.polygon, point))
    }
    pub fn nearest_vertex(&self, point: Complex) -> Option<&Vertex> {
        self.vertices.iter().min_by(|a, b| {
            (a.center - point)
                .norm_sq()
                .total_cmp(&(b.center - point).norm_sq())
        })
    }
}
fn segment_distance(p: Complex, a: Complex, b: Complex) -> f64 {
    let v = b - a;
    let t = if v.norm_sq() > 0.0 {
        ((p - a).dot(v) / v.norm_sq()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (p - (a + v * t)).norm()
}
/// Ear clipping works for the curved, generally non-convex Euclidean polygons.
pub fn triangulate(poly: &[Complex]) -> Vec<[usize; 3]> {
    let area: f64 = poly
        .iter()
        .zip(poly.iter().cycle().skip(1))
        .take(poly.len())
        .map(|(&a, &b)| a.cross(b))
        .sum();
    let sign = area.signum();
    let mut remaining: Vec<_> = (0..poly.len()).collect();
    let mut result = Vec::new();
    while remaining.len() > 2 {
        let mut ear = None;
        for i in 0..remaining.len() {
            let a = remaining[(i + remaining.len() - 1) % remaining.len()];
            let b = remaining[i];
            let c = remaining[(i + 1) % remaining.len()];
            let cross = (poly[b] - poly[a]).cross(poly[c] - poly[b]) * sign;
            if cross <= 1e-16 {
                continue;
            }
            let contains = remaining.iter().any(|&j| {
                j != a
                    && j != b
                    && j != c
                    && [
                        (poly[b] - poly[a]).cross(poly[j] - poly[a]),
                        (poly[c] - poly[b]).cross(poly[j] - poly[b]),
                        (poly[a] - poly[c]).cross(poly[j] - poly[c]),
                    ]
                    .iter()
                    .all(|v| v * sign > 1e-16)
            });
            if !contains {
                ear = Some((i, [a, b, c]));
                break;
            }
        }
        if let Some((i, t)) = ear {
            result.push(t);
            remaining.remove(i);
        } else {
            break;
        }
    }
    result
}
