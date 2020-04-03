use crate::physics::bounding_box::{Aabb, Ray};
use glam::Vec3;
#[allow(unused_imports)]
use log::{error, info, trace};

// a bit of margin :D
const EPSILON: f32 = 0.5;

fn eq(a: f32, b: f32) -> bool {
    (a - b).abs() < EPSILON
}

/// Return normal of contact if there is contact.
pub fn generate_contacts(a: &Aabb, va: &Vec3, b: &Aabb, vb: &Vec3) -> Option<(f32, Vec3)> {
    let v = *va - *vb;

    let bigger_box = Aabb::new(b.center, a.halfwidths + b.halfwidths);

    let ray = Ray::new(a.center, v);

    trace!("bounding box of a is {:?}", a);
    trace!("bounding box of b is {:?}", b);
    trace!("Resulting box is {:?}", bigger_box);
    trace!("Ray is {:?}", ray);
    trace!("Result is {:?}", bigger_box.interset_ray(ray));

    if let Some((t, point)) = bigger_box.interset_ray(ray) {
        let min = bigger_box.center - bigger_box.halfwidths;
        let max = bigger_box.center + bigger_box.halfwidths;
        if eq(point.x(), min.x()) {
            Some((t, glam::vec3(1.0, 0.0, 0.)))
        } else if eq(point.x(), max.x()) {
            Some((t, glam::vec3(1.0, 0.0, 0.0)))
        } else if eq(point.y(), min.y()) {
            Some((t, glam::vec3(0.0, 1., 0.)))
        } else if eq(point.y(), max.y()) {
            Some((t, glam::vec3(0., 1., 0.)))
        } else if eq(point.z(), min.z()) {
            Some((t, glam::vec3(0., 0., 1.)))
        } else if eq(point.z(), max.z()) {
            Some((t, glam::vec3(0., 0., 1.)))
        } else {
            dbg!(v);
            dbg!(t, point, max, min);
            dbg!(a, va, b, vb);

            error!("Boxes are overlapping");
            None
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::physics2::bounding_box::Aabb;
    #[test]
    fn test1() {
        // box from the top.
        let a = Aabb::new(Vec3::new(0.0, 5.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        let va = Vec3::new(0.0, -2.0, 0.0);

        // static box at the bottom
        let b = Aabb::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 3.0, 1.0));
        let vb = Vec3::zero();

        dbg!(generate_contacts(&a, &va, &b, &vb));
    }
}
