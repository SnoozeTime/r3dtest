use nalgebra::UnitQuaternion;

pub fn quat_to_direction(q: glam::Quat) -> (glam::Vec3, glam::Vec3, glam::Vec3) {
    /*
    forward vector:
        x = 2 * (x*z + w*y)
        y = 2 * (y*z - w*x)
        z = 1 - 2 * (x*x + y*y)

        up vector
        x = 2 * (x*y - w*z)
        y = 1 - 2 * (x*x + z*z)
        z = 2 * (y*z + w*x)

        left vector
        x = 1 - 2 * (y*y + z*z)
        y = 2 * (x*y + w*z)
        z = 2 * (x*z - w*y)
    */
    let [x, y, z, w]: [f32; 4] = q.into();
    let front = glam::vec3(
        2.0 * (x * z + w * y),
        2.0 * (y * z - w * x),
        1.0 - 2.0 * (x * x + y * y),
    );
    let up = glam::vec3(
        2.0 * (x * y - w * z),
        1.0 - 2.0 * (x * x + z * z),
        2.0 * (y * z + w * x),
    );
    let left = glam::vec3(
        1.0 - 2.0 * (y * y + z * z),
        2.0 * (x * y + w * z),
        2.0 * (x * z - w * y),
    );

    (front, up, left)
}

pub fn quat_to_euler(q: glam::Quat) -> glam::Vec3 {
    let (axis, angle) = q.to_axis_angle();
    let rot = UnitQuaternion::from_scaled_axis(
        nalgebra::Vector3::new(axis.x(), axis.y(), axis.z()) * angle,
    );
    let (roll, pitch, yaw) = rot.euler_angles();
    glam::Vec3::new(yaw, pitch, roll)
}
