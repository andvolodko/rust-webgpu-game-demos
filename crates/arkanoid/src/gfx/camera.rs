//! Камера: 3/4-вид, дистанція підганяється під аспект, щоб поле не обрізалось.

use crate::game::{PADDLE_Y, WORLD_H, WORLD_W};
use engine::math::{game_to_world, Mat4, Vec3};

const FOV_Y: f32 = 38.0_f32;
const NEAR: f32 = 30.0;
const FAR: f32 = 2800.0;
const UP: Vec3 = Vec3 {
    x: 0.0,
    y: 0.0,
    z: 1.0,
};

pub struct Camera {
    pub eye: Vec3,
    pub view: Mat4,
    pub view_proj: Mat4,
}

fn gw(x: f32, y: f32, z: f32) -> Vec3 {
    game_to_world(x, y, z, WORLD_H)
}

fn playfield_corners() -> [Vec3; 8] {
    let z_hi = 24.0;
    [
        gw(-8.0, -8.0, 0.0),
        gw(WORLD_W + 8.0, -8.0, 0.0),
        gw(-8.0, WORLD_H + 12.0, 0.0),
        gw(WORLD_W + 8.0, WORLD_H + 12.0, 0.0),
        gw(-8.0, -8.0, z_hi),
        gw(WORLD_W + 8.0, -8.0, z_hi),
        gw(-8.0, PADDLE_Y + 20.0, z_hi),
        gw(WORLD_W + 8.0, PADDLE_Y + 20.0, z_hi),
    ]
}

fn ndc_overflow(vp: Mat4, p: Vec3, max_x: f32, max_y: f32) -> f32 {
    let c = vp.mul_vec4(p.x, p.y, p.z, 1.0);
    if c[3] <= 1e-4 {
        return 2.0;
    }
    let x = (c[0] / c[3]).abs() - max_x;
    let y = (c[1] / c[3]).abs() - max_y;
    x.max(y).max(0.0)
}

/// Підганяє дистанцію камери так, щоб AABB поля + paddle були в кадрі.
pub fn fit(aspect: f32, shake_x: f32, shake_z: f32, lean_x: f32, lean_y: f32) -> Camera {
    let target = Vec3::new(
        WORLD_W * 0.5 + lean_x * 48.0,
        WORLD_H * 0.36 + lean_y * 30.0,
        6.0,
    );
    let to_eye = Vec3::new(lean_x * 0.14, -0.56, 0.82).normalized();
    let fov = FOV_Y.to_radians();
    let corners = playfield_corners();
    let max_x = 0.96;
    let max_y = 0.86;

    let mut dist = 620.0;
    for _ in 0..16 {
        let eye = target.add(to_eye.scale(dist));
        let vp = Mat4::perspective_rh(fov, aspect, NEAR, FAR).mul(Mat4::look_at_rh(eye, target, UP));
        let mut overflow = 0.0_f32;
        for p in corners {
            overflow = overflow.max(ndc_overflow(vp, p, max_x, max_y));
        }
        if overflow <= 0.002 {
            dist *= 0.985;
        } else {
            dist *= 1.0 + overflow * 0.65;
        }
        dist = dist.clamp(380.0, 1600.0);
    }

    let eye = Vec3::new(
        target.x + to_eye.x * dist + shake_x,
        target.y + to_eye.y * dist,
        target.z + to_eye.z * dist + shake_z,
    );
    let view = Mat4::look_at_rh(eye, target, UP);
    Camera {
        eye,
        view,
        view_proj: Mat4::perspective_rh(fov, aspect, NEAR, FAR).mul(view),
    }
}
