//! Some utilities to work with geometry, vectors, quaternions and stuff.
//! from here https://en.wikipedia.org/wiki/Conversion_between_quaternions_and_Euler_angles

pub fn quat_from_euler(yaw: f32, pitch: f32, roll: f32) -> glam::Quat {
    let cy = (yaw * 0.5).cos();
    let sy = (yaw * 0.5).sin();
    let cp = (pitch * 0.5).cos();
    let sp = (pitch * 0.5).sin();
    let cr = (roll * 0.5).cos();
    let sr = (roll * 0.5).sin();

    let w = cr * cp * cy + sr * sp * sy;
    let x = sr * cp * cy - cr * sp * sy;
    let y = cr * sp * cy + sr * cp * sy;
    let z = cr * cp * sy - sr * sp * cy;

    glam::Quat::from_xyzw(x, y, z, w)
}

pub fn euler_from_quat(q: glam::Quat) -> (f32, f32, f32) {
    let [qx, qy, qz, qw]: [f32; 4] = q.into();
    // roll (x-axis rotation)
    let sinr_cosp = 2.0 * (qw * qx + qy * qz);
    let cosr_cosp = 1.0 - 2.0 * (qx * qx + qy * qy);
    let roll = sinr_cosp.atan2(cosr_cosp); //std::atan2(sinr_cosp, cosr_cosp);

    let sinp = 2.0 * (qw * qy - qz * qx);
    let pitch = if sinp.abs() >= 1.0 {
        sinp.signum() * std::f32::consts::PI
    } else {
        sinp.asin()
    };

    // yaw (z-axis rotation)
    let siny_cosp = 2.0 * (qw * qz + qx * qy);
    let cosy_cosp = 1.0 - 2.0 * (qy * qy + qz * qz);
    let yaw = siny_cosp.atan2(cosy_cosp);

    (yaw, pitch, roll)
}

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
