use glam::Quat;
/**
[




'[Point3::new((-1.0, -1.0, -1.0)),
Point3::new((-1.0, -1.0, -0.49584853649139404)),
Point3::new((-1.0, 1.0, -0.49584853649139404)),
Point3::new((-1.0, 1.0, -1.0))]',

'[Point3::new((0.49584853649139404, -1.0,1.0)),
Point3::new((0.49584853649139404, 1.0, 1.0)),
Point3::new((-1.0, 1.0, -0.49584853649139404)),
Point3::new((-1.0, -1.0, -0.49584853649139404))]',

'[Point3::new((-1.0, 1.0, -1.0)),
Point3::new((-1.0, 1.0, -0.49584853649139404)),
Point3::new((0.49584853649139404, 1.0, 1.0)),
Point3::new((1.0, 1.0, 1.0)),
Point3::new((1.0, 1.0, -1.0))]
**/
use nalgebra::geometry::Point3;
use ncollide3d::shape::ConvexHull;

fn main() {
    println!("{:?}", Quat::from_rotation_z(0.4).normalize());
}
