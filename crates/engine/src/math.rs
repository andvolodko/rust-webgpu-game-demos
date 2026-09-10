/// Мінімалістична 2D/3D-математика: Vec2, Vec3, Mat4, AABB, колізії.
/// Без зовнішніх залежностей (glam тощо) — щоб тримати WASM маленьким.

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn add(self, o: Self) -> Self {
        Self::new(self.x + o.x, self.y + o.y)
    }

    pub fn sub(self, o: Self) -> Self {
        Self::new(self.x - o.x, self.y - o.y)
    }

    pub fn scale(self, s: f32) -> Self {
        Self::new(self.x * s, self.y * s)
    }

    pub fn dot(self, o: Self) -> f32 {
        self.x * o.x + self.y * o.y
    }

    pub fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    pub fn normalized(self) -> Self {
        let len = self.length();
        if len > 1e-6 {
            self.scale(1.0 / len)
        } else {
            Self::ZERO
        }
    }
}

/// 3D-вектор (світ: X праворуч, Y уздовж поля вгору, Z висота).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn add(self, o: Self) -> Self {
        Self::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }

    pub fn sub(self, o: Self) -> Self {
        Self::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }

    pub fn scale(self, s: f32) -> Self {
        Self::new(self.x * s, self.y * s, self.z * s)
    }

    pub fn dot(self, o: Self) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    pub fn cross(self, o: Self) -> Self {
        Self::new(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }

    pub fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    pub fn normalized(self) -> Self {
        let len = self.length();
        if len > 1e-6 {
            self.scale(1.0 / len)
        } else {
            Self::ZERO
        }
    }
}

/// Стовпчикова 4×4 матриця (WGSL / wgpu).
#[derive(Debug, Clone, Copy)]
pub struct Mat4 {
    pub m: [f32; 16],
}

impl Mat4 {
    #[allow(dead_code)]
    pub const IDENTITY: Self = Self {
        m: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    };

    pub fn from_cols_array_2d(cols: [[f32; 4]; 4]) -> Self {
        let mut m = [0.0f32; 16];
        for col in 0..4 {
            for row in 0..4 {
                m[col * 4 + row] = cols[col][row];
            }
        }
        Self { m }
    }

    pub fn translation(t: Vec3) -> Self {
        Self {
            m: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, t.x, t.y, t.z, 1.0,
            ],
        }
    }

    pub fn from_scale(s: Vec3) -> Self {
        Self {
            m: [
                s.x, 0.0, 0.0, 0.0, 0.0, s.y, 0.0, 0.0, 0.0, 0.0, s.z, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    pub fn rotation_x(rad: f32) -> Self {
        let (s, c) = rad.sin_cos();
        Self {
            m: [
                1.0, 0.0, 0.0, 0.0, 0.0, c, s, 0.0, 0.0, -s, c, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    pub fn rotation_y(rad: f32) -> Self {
        let (s, c) = rad.sin_cos();
        Self {
            m: [
                c, 0.0, -s, 0.0, 0.0, 1.0, 0.0, 0.0, s, 0.0, c, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    pub fn rotation_z(rad: f32) -> Self {
        let (s, c) = rad.sin_cos();
        Self {
            m: [
                c, s, 0.0, 0.0, -s, c, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    pub fn transform_point(self, p: Vec3) -> Vec3 {
        let v = self.mul_vec4(p.x, p.y, p.z, 1.0);
        let w = if v[3].abs() > 1e-8 { v[3] } else { 1.0 };
        Vec3::new(v[0] / w, v[1] / w, v[2] / w)
    }

    pub fn transform_vector(self, v: Vec3) -> Vec3 {
        let t = self.mul_vec4(v.x, v.y, v.z, 0.0);
        Vec3::new(t[0], t[1], t[2])
    }

    pub fn mul(self, o: Self) -> Self {
        let a = &self.m;
        let b = &o.m;
        let mut r = [0.0f32; 16];
        for col in 0..4 {
            for row in 0..4 {
                r[col * 4 + row] = a[row] * b[col * 4]
                    + a[4 + row] * b[col * 4 + 1]
                    + a[8 + row] * b[col * 4 + 2]
                    + a[12 + row] * b[col * 4 + 3];
            }
        }
        Self { m: r }
    }

    /// Стовпчикова `M * (x,y,z,w)`.
    pub fn mul_vec4(self, x: f32, y: f32, z: f32, w: f32) -> [f32; 4] {
        let m = &self.m;
        [
            m[0] * x + m[4] * y + m[8] * z + m[12] * w,
            m[1] * x + m[5] * y + m[9] * z + m[13] * w,
            m[2] * x + m[6] * y + m[10] * z + m[14] * w,
            m[3] * x + m[7] * y + m[11] * z + m[15] * w,
        ]
    }

    /// Right-handed perspective, clip Z у [0, 1] (WebGPU).
    pub fn perspective_rh(fov_y: f32, aspect: f32, znear: f32, zfar: f32) -> Self {
        let f = 1.0 / (fov_y * 0.5).tan();
        let a = aspect.max(0.0001);
        let mut m = [0.0f32; 16];
        m[0] = f / a;
        m[5] = f;
        m[10] = zfar / (znear - zfar);
        m[11] = -1.0;
        m[14] = (znear * zfar) / (znear - zfar);
        Self { m }
    }

    /// Right-handed ortho, clip Z у [0, 1] (WebGPU).
    pub fn orthographic_rh(half_w: f32, half_h: f32, znear: f32, zfar: f32) -> Self {
        let mut m = [0.0f32; 16];
        m[0] = 1.0 / half_w.max(0.0001);
        m[5] = 1.0 / half_h.max(0.0001);
        m[10] = 1.0 / (znear - zfar);
        m[14] = znear / (znear - zfar);
        m[15] = 1.0;
        Self { m }
    }

    /// Right-handed look-at (камера дивиться на target).
    pub fn look_at_rh(eye: Vec3, target: Vec3, up: Vec3) -> Self {
        let f = target.sub(eye).normalized();
        let s = f.cross(up).normalized();
        let u = s.cross(f);
        // f — напрям вперед; у RH view-space камера дивиться вздовж -Z,
        // тому третій стовпець = -f.
        Self {
            m: [
                s.x,
                u.x,
                -f.x,
                0.0,
                s.y,
                u.y,
                -f.y,
                0.0,
                s.z,
                u.z,
                -f.z,
                0.0,
                -s.dot(eye),
                -u.dot(eye),
                f.dot(eye),
                1.0,
            ],
        }
    }
}

/// Ігрові XY (Y вниз) → світ XYZ (Y уздовж поля вгору, Z висота).
pub fn game_to_world(x: f32, y: f32, z: f32, world_h: f32) -> Vec3 {
    Vec3::new(x, world_h - y, z)
}

/// Вісь-вирівняний прямокутник: центр + половинні розміри.
#[derive(Debug, Clone, Copy)]
pub struct Aabb {
    pub center: Vec2,
    pub half: Vec2,
}

impl Aabb {
    pub const fn new(center: Vec2, half: Vec2) -> Self {
        Self { center, half }
    }

    pub fn min(self) -> Vec2 {
        Vec2::new(self.center.x - self.half.x, self.center.y - self.half.y)
    }

    pub fn max(self) -> Vec2 {
        Vec2::new(self.center.x + self.half.x, self.center.y + self.half.y)
    }

    #[allow(dead_code)]
    pub fn contains_x(self, x: f32) -> bool {
        x >= self.min().x && x <= self.max().x
    }

    #[allow(dead_code)]
    pub fn contains_y(self, y: f32) -> bool {
        y >= self.min().y && y <= self.max().y
    }
}

/// Результат колізії кола з прямокутником: нормаль відбиття + глибина проникнення.
#[derive(Debug, Clone, Copy)]
pub struct Hit {
    pub normal: Vec2,
    pub depth: f32,
}

/// Колізія кола (центр + радіус) з AABB. Повертає None, якщо коло поза AABB
/// (або тільки торкається його — глибина <= 0).
pub fn circle_aabb(center: Vec2, radius: f32, aabb: Aabb) -> Option<Hit> {
    let closest_x = clamp(center.x, aabb.min().x, aabb.max().x);
    let closest_y = clamp(center.y, aabb.min().y, aabb.max().y);
    let dx = center.x - closest_x;
    let dy = center.y - closest_y;
    let dist_sq = dx * dx + dy * dy;
    if dist_sq > radius * radius {
        return None;
    }

    let dist = dist_sq.sqrt();
    if dist > 1e-6 {
        // Центр кола поза прямокутником: нормаль = напрямок на центр.
        let normal = Vec2::new(dx / dist, dy / dist);
        Some(Hit {
            normal,
            depth: radius - dist,
        })
    } else {
        // Центр всередині прямокутника: штовхаємо по осі найменшого проникнення.
        let overlap_x = aabb.half.x + radius - (center.x - aabb.center.x).abs();
        let overlap_y = aabb.half.y + radius - (center.y - aabb.center.y).abs();
        if overlap_x < overlap_y {
            let sx = if center.x >= aabb.center.x { 1.0 } else { -1.0 };
            Some(Hit {
                normal: Vec2::new(sx, 0.0),
                depth: overlap_x,
            })
        } else {
            let sy = if center.y >= aabb.center.y { 1.0 } else { -1.0 };
            Some(Hit {
                normal: Vec2::new(0.0, sy),
                depth: overlap_y,
            })
        }
    }
}

/// Перетин рухомого по X кола з AABB ЕСЛИ коло вже перетинається по Y —
/// використовується платформою.
pub fn clamp(v: f32, lo: f32, hi: f32) -> f32 {
    if v < lo {
        lo
    } else if v > hi {
        hi
    } else {
        v
    }
}
