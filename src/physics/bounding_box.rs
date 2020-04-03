//! Bounding volumes are AABB (axis-aligned).
use glam::Vec3;
use std::mem::swap;

const EPSILON: f32 = 0.0001;

#[derive(Debug, Copy, Clone)]
pub struct Ray {
    /// origin of ray
    pub c: Vec3,

    /// direction of ray
    pub d: Vec3,
}

impl Ray {
    pub fn new(c: Vec3, d: Vec3) -> Self {
        Self { c, d }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Aabb {
    /// Center of the AABB
    pub center: Vec3,

    /// w/2, h/2, l/2
    pub halfwidths: Vec3,
}

impl Aabb {
    pub fn new(center: Vec3, halfwidths: Vec3) -> Self {
        Self { center, halfwidths }
    }
    /// Test if the two AABBs are intersecting.
    ///
    /// returns true if that is the case
    pub fn intersect(&self, other: &Aabb) -> bool {
        if (self.center.x() - other.center.x()).abs() > (self.halfwidths.x() + other.halfwidths.x())
        {
            return false;
        }

        if (self.center.y() - other.center.y()).abs() > (self.halfwidths.y() + other.halfwidths.y())
        {
            return false;
        }

        if (self.center.z() - other.center.z()).abs() > (self.halfwidths.z() + other.halfwidths.z())
        {
            return false;
        }

        true
    }

    /// Check if the two moving AABBs are intersecting.
    ///
    /// va is the speed of `self`, vb is the speed of the other aabb.
    /// Returns time of first impact and time of last impact.
    pub fn intersect_moving(&self, other: &Aabb, va: Vec3, vb: Vec3) -> Option<(f32, f32)> {
        if self.intersect(other) {
            return Some((0.0, 0.0));
        }

        let mut tmin = 0.0f32;
        let mut tmax = 1.0f32;

        let amax = self.center + self.halfwidths;
        let amin = self.center - self.halfwidths;
        let bmax = other.center + other.halfwidths;
        let bmin = other.center - other.halfwidths;

        let v = vb - va; // relative velocity. Now the test is a dynamic against a static.
        for i in 0..3 {
            if v[i] < 0.0 {
                if bmax[i] < amax[i] {
                    return None;
                }
                if amax[i] < bmin[i] {
                    tmin = tmin.max((amax[i] - bmin[i]) / v[i]);
                }
                if bmax[i] < amin[i] {
                    tmax = tmax.min((amin[i] - bmax[i]) / v[i]);
                }
            }
            if v[i] > 0.0 {
                // non-intersecting and moving apart.
                if bmin[i] > amax[i] {
                    return None;
                }
                if bmax[i] < amin[i] {
                    tmin = tmin.max((amin[i] - bmax[i]) / v[i]);
                }
                if amax[i] > bmin[i] {
                    tmax = tmax.min((amax[i] - bmin[i]) / v[i]);
                }
            }

            if tmin > tmax {
                return None;
            }
        }

        Some((tmin, tmax))
    }

    /// Compute the closest point on (or in) the AABB to p.
    ///
    /// It works by clamping p-coordinates to the bounds of the AABB
    pub fn closest_point(&self, p: Vec3) -> Vec3 {
        let min = self.center - self.halfwidths;
        let max = self.center + self.halfwidths;
        let q_x = (p.x().max(min.x())).min(max.x());
        let q_y = (p.y().max(min.y())).min(max.y());
        let q_z = (p.z().max(min.z())).min(max.z());
        Vec3::new(q_x, q_y, q_z)
    }

    /// Compute distance between point and AABB.
    pub fn square_distance_to_point(&self, p: Vec3) -> f32 {
        let mut sqr_dist = 0.0;
        let min = self.center - self.halfwidths;
        let max = self.center + self.halfwidths;

        if p.x() < min.x() {
            sqr_dist += (min.x() - p.x()).exp2();
        } else if p.x() > max.x() {
            sqr_dist += (p.x() - max.x()).exp2();
        }
        if p.y() < min.y() {
            sqr_dist += (min.y() - p.y()).exp2();
        } else if p.y() > max.y() {
            sqr_dist += (p.y() - max.y()).exp2();
        }
        if p.z() < min.z() {
            sqr_dist += (min.z() - p.z()).exp2();
        } else if p.z() > max.z() {
            sqr_dist += (p.z() - max.z()).exp2();
        }

        sqr_dist
    }

    /// Check if ray intersects the AABB. If yes, it will return the time of intersection and point
    /// of intersection;
    ///
    /// # Algorithm
    /// Check the intersection of ray with each slabs of the AABB (x-axis slab, y-axis, z-axis).
    /// If the intersections overlap, then the ray intersects with the AABB (recall, a point is
    /// in the AABB if it is in the three slabs).
    ///
    /// For each slab, compute the time of entry and the time of exit. Then, take the max of time
    /// of entry, take the min of time of exit. If t_entry < t_exit, the slabs overlap.
    ///
    /// Ray equation: R(t) = P + t.d where P is origin of ray and d its direction.
    /// Equation of planes: X.ni = di.
    /// Substitute X by R to get the intersection.
    /// (P + t.d) . ni = di
    /// t = (di - P.ni)/(d.ni)
    ///
    /// For the AABB planes, n is along the axis. The expression can be simplified: for example
    /// t = (d - px)/dx where d is the position of the plane along the x axis.
    pub fn interset_ray(&self, ray: Ray) -> Option<(f32, Vec3)> {
        let mut tmin = 0.0f32; // set to -FLT_MAX to get first hit on the line.
        let mut tmax = std::f32::MAX; // max distance the ray can travel.

        let min = self.center - self.halfwidths;
        let max = self.center + self.halfwidths;

        for i in 0..3 {
            if ray.d[i].abs() < EPSILON {
                // ray is parallel to the slab so we only need to test whether the origin is within
                // the slab.
                if ray.c[i] < min[i] || ray.c[i] > max[i] {
                    return None;
                }
            } else {
                let ood = 1.0 / ray.d[i];
                let mut t1 = (min[i] - ray.c[i]) * ood;
                let mut t2 = (max[i] - ray.c[i]) * ood;

                // make t1 intersection with the near plane.
                if t2 < t1 {
                    swap(&mut t2, &mut t1);
                }

                // compute intersection of slabs intersection intervals.
                // farthest of all entries.
                if t1 > tmin {
                    tmin = t1;
                }
                // nearest of all exits
                if t2 < tmax {
                    tmax = t2;
                }

                if tmin > tmax {
                    return None;
                }
            }
        }

        let q = ray.c + tmin * ray.d;

        Some((tmin, q))
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn simple_intersection_test() {
        // A is at the origin.
        let a = Aabb::new(Vec3::zero(), glam::vec3(1.0, 1.0, 1.0));

        // do not intersect (LEFT X)
        let b = Aabb::new(glam::vec3(-5.0, 0.0, 0.0), glam::vec3(1.0, 1.0, 1.0));
        assert!(!a.intersect(&b));
        // do not intersect (RIGHT X)
        let b = Aabb::new(glam::vec3(5.0, 0.0, 0.0), glam::vec3(1.0, 1.0, 1.0));
        assert!(!a.intersect(&b));
        // do not intersect (Y)
        let b = Aabb::new(glam::vec3(0.0, 5.0, 0.0), glam::vec3(1.0, 1.0, 1.0));
        assert!(!a.intersect(&b));
        let b = Aabb::new(glam::vec3(0.0, -5.0, 0.0), glam::vec3(1.0, 1.0, 1.0));
        assert!(!a.intersect(&b));
        // do not intersect (z)
        let b = Aabb::new(glam::vec3(0.0, 0.0, 5.0), glam::vec3(1.0, 1.0, 1.0));
        assert!(!a.intersect(&b));
        let b = Aabb::new(glam::vec3(0.0, 0.0, -5.0), glam::vec3(1.0, 1.0, 1.0));
        assert!(!a.intersect(&b));

        // Intersect.
        let b = Aabb::new(glam::vec3(-5.0, 0.0, 0.0), glam::vec3(5.0, 1.0, 1.0));
        assert!(a.intersect(&b));
        let b = Aabb::new(Vec3::zero(), glam::vec3(2.0, 2.0, 2.0));
        assert!(a.intersect(&b));

        // face overlap.
        let b = Aabb::new(glam::vec3(2.0, 0.0, 0.0), glam::vec3(1.0, 1.0, 1.0));
        assert!(a.intersect(&b));
    }

    #[test]
    fn closest_point_test() {
        // Point inside.
        let a = Aabb::new(Vec3::zero(), glam::vec3(1.0, 1.0, 1.0));
        assert_eq!(Vec3::zero(), a.closest_point(Vec3::zero()));

        // points on faces. (voronoid region of face)
        assert_eq!(
            glam::vec3(1.0, 0.3, 0.0),
            a.closest_point(glam::vec3(5.0, 0.3, 0.0))
        );
        assert_eq!(
            glam::vec3(-1.0, 0.3, 0.0),
            a.closest_point(glam::vec3(-5.0, 0.3, 0.0))
        );

        // points on edges
        assert_eq!(
            glam::vec3(1.0, 1., 0.0),
            a.closest_point(glam::vec3(5.0, 5.3, 0.0))
        );
        assert_eq!(
            glam::vec3(-1.0, -1.0, 0.0),
            a.closest_point(glam::vec3(-5.0, -50.3, 0.0))
        );

        // Vertex.
        assert_eq!(
            glam::vec3(1.0, 1.0, 1.0),
            a.closest_point(glam::vec3(5.0, 5.3, 5.0))
        );
    }

    #[test]
    fn distance_to_point_test() {
        // Point inside.
        let a = Aabb::new(Vec3::zero(), glam::vec3(1.0, 1.0, 1.0));
        assert_eq!(0.0, a.square_distance_to_point(Vec3::zero()));

        // points on faces. (voronoid region of face)
        assert_eq!(16.0, a.square_distance_to_point(glam::vec3(5.0, 0.3, 0.0)));
        assert_eq!(16.0, a.square_distance_to_point(glam::vec3(-5.0, 0.3, 0.0)));

        // points on edges
        assert_eq!(32.0, a.square_distance_to_point(glam::vec3(5.0, 5.0, 0.0)));
    }

    #[test]
    fn ray_intersect_test() {
        let a = Aabb::new(Vec3::zero(), glam::vec3(1.0, 1.0, 1.0));
        let ray = Ray::new(glam::vec3(-5.0, 0.0, 0.0), glam::vec3(1.0, 0.0, 0.0));

        if let Some((t, q)) = a.interset_ray(ray) {
            assert_eq!(q, glam::vec3(-1.0, 0.0, 0.0));
            assert_eq!(t, 4.0);
        } else {
            assert!(false, "Ray should collide.");
        }

        let ray = Ray::new(glam::vec3(-5.0, -5.0, 0.0), glam::vec3(1.0, 1.0, 0.0));
        // should intersect at the center of an edge.
        if let Some((t, q)) = a.interset_ray(ray) {
            assert_eq!(q, glam::vec3(-1.0, -1.0, 0.0));
            assert_eq!(t, 4.0);
        } else {
            assert!(false, "Ray should collide.");
        }

        let ray = Ray::new(glam::vec3(0.0, -5.0, -6.0), glam::vec3(0.0, 1.0, 1.0));
        // should intersect at the center of an edge.
        if let Some((t, q)) = a.interset_ray(ray) {
            assert_eq!(q, glam::vec3(0.0, 0.0, -1.0));
            assert_eq!(t, 5.0);
        } else {
            assert!(false, "Ray should collide.");
        }
    }

    #[test]
    fn intersect_moving_test() {
        let a = Aabb::new(Vec3::zero(), glam::vec3(1.0, 1.0, 1.0));
        let b = Aabb::new(Vec3::new(-3.0, 0.0, 0.0), glam::vec3(1.0, 1.0, 1.0));

        // b moves at a speed of 5 along the x axis. Should intersect with a.
        if let Some((tmin, tmax)) = a.intersect_moving(&b, Vec3::zero(), glam::vec3(5.0, 0.0, 0.0))
        {
            // distance is 1. so at speed 5 the entry time is 0.2;
            // exit time is 5 (2 length + 1 the distance) / 5 (speed)
            assert_eq!(tmin, 0.2);
            assert_eq!(tmax, 1.0);
        } else {
            assert!(false);
        }
    }
}
