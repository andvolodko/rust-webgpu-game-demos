/// GPU-симуляція партіклів: гравітація, drag, bounce, fade.
struct Particle {
    pos: vec3<f32>,
    life: f32,
    vel: vec3<f32>,
    max_life: f32,
    angles: vec3<f32>,
    scale: f32,
    color: vec4<f32>,
    ang_vel: vec3<f32>,
    bounce: f32,
};

struct Sim {
    dt: f32,
    gravity: f32,
    drag: f32,
    _pad: f32,
};

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> sim: Sim;

@compute @workgroup_size(64)
fn simulate(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i >= arrayLength(&particles)) {
        return;
    }
    var p = particles[i];
    if (p.life <= 0.0) {
        return;
    }
    p.life -= sim.dt;
    if (p.life <= 0.0) {
        p.life = 0.0;
        particles[i] = p;
        return;
    }
    p.vel.z -= sim.gravity * sim.dt;
    p.vel *= sim.drag;
    p.pos += p.vel * sim.dt;
    p.angles += p.ang_vel * sim.dt;
    if (p.bounce > 0.5) {
        let floor_z = p.scale * 0.4;
        if (p.pos.z < floor_z) {
            p.pos.z = floor_z;
            p.vel.z *= -0.32;
            p.vel.x *= 0.72;
            p.vel.y *= 0.72;
        }
    }
    particles[i] = p;
}
