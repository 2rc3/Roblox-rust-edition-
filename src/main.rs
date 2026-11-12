use macroquad::prelude::*;

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Solid,
    Lava,
    Check,
    Win,
}

struct Part {
    p: Vec3, // center
    s: Vec3, // full size
    c: Color,
    k: Kind,
}

fn part(p: Vec3, s: Vec3, c: Color, k: Kind) -> Part {
    Part { p, s, c, k }
}

// Is the player box (center pp, half-size ph) touching part q?
fn touching(pp: Vec3, ph: Vec3, q: &Part, pad: f32) -> bool {
    let h = q.s * 0.5 + ph + Vec3::splat(pad);
    let d = (pp - q.p).abs();
    d.x < h.x && d.y < h.y && d.z < h.z
}

fn blocked(w: &[Part], p: Vec3, ph: Vec3) -> bool {
    w.iter().any(|q| q.k != Kind::Lava && touching(p, ph, q, 0.0))
}

fn course() -> Vec<Part> {
    let mut w = vec![
        part(vec3(0., -1., 0.), vec3(24., 2., 24.), GREEN, Kind::Solid),
        part(vec3(0., -6., 40.), vec3(120., 1., 160.), ORANGE, Kind::Lava),
    ];
    let pal = [PURPLE, BLUE, PINK, VIOLET, BROWN, MAGENTA];
    let n = 12;
    for i in 0..n {
        let f = i as f32;
        let pos = vec3((f * 1.1).sin() * 2.5, 0.3 + f * 0.7, 16. + f * 4.5);
        let k = if i == n - 1 {
            Kind::Win
        } else if i % 4 == 3 {
            Kind::Check
        } else {
            Kind::Solid
        };
        let c = match k {
            Kind::Win => GOLD,
            Kind::Check => WHITE,
            _ => pal[i % pal.len()],
        };
        w.push(part(pos, vec3(3.4, 1., 3.4), c, k));
        if k == Kind::Solid && i % 3 == 2 {
            // kill brick across the platform: jump over it!
            w.push(part(pos + vec3(0., 0.65, 0.), vec3(3.4, 0.3, 0.7), RED, Kind::Lava));
        }
    }
    w
}

fn btn(c: Vec2, r: f32, t: &str) {
    draw_circle(c.x, c.y, r, Color::new(1., 1., 1., 0.3));
    let fs = (r * 0.45) as u16;
    let d = measure_text(t, None, fs, 1.0);
    draw_text(t, c.x - d.width / 2., c.y + d.height / 2., fs as f32, WHITE);
}

#[macroquad::main("BlockVerse")]
async fn main() {
    let mut world = course();
    let base = world.len();
    let ph = vec3(0.4, 0.9, 0.4);
    let start = vec3(0., 1., 0.);
    let (mut spawn, mut pl, mut vel) = (start, start, Vec3::ZERO);
    let (mut yaw, mut pitch) = (std::f32::consts::PI, 0.5f32);
    let (mut build, mut win_t) = (false, 0.0f32);
    let mut cam_touch = None;

    loop {
        let (w, h) = (screen_width(), screen_height());
        let u = h / 720.;
        let dt = get_frame_time().min(1. / 30.);
        let joy = vec2(150. * u, h - 160. * u);
        let jump_b = vec2(w - 130. * u, h - 140. * u);
        let build_b = vec2(w - 80. * u, 80. * u);
        let plus_b = vec2(w - 80. * u, 210. * u);
        let minus_b = vec2(w - 80. * u, 340. * u);

        // ---- input (touch + keyboard) ----
        let mut stick = Vec2::ZERO;
        let (mut jump, mut toggle, mut place, mut remove) = (false, false, false, false);
        for t in touches() {
            let p = t.position;
            let started = matches!(t.phase, TouchPhase::Started);
            let ended = matches!(t.phase, TouchPhase::Ended | TouchPhase::Cancelled);
            if !ended && p.distance(joy) < 180. * u {
                stick = (p - joy) / (100. * u);
            } else if started && p.distance(jump_b) < 75. * u {
                jump = true;
            } else if started && p.distance(build_b) < 55. * u {
                toggle = true;
            } else if started && build && p.distance(plus_b) < 55. * u {
                place = true;
            } else if started && build && p.distance(minus_b) < 55. * u {
                remove = true;
            } else if started {
                cam_touch = Some((t.id, p));
            } else if let Some((id, last)) = cam_touch {
                if id == t.id {
                    if ended {
                        cam_touch = None;
                    } else {
                        yaw -= (p.x - last.x) * 0.006;
                        pitch = (pitch + (p.y - last.y) * 0.006).clamp(-0.1, 1.3);
                        cam_touch = Some((id, p));
                    }
                }
            }
        }
        if is_key_down(KeyCode::A) { stick.x -= 1.; }
        if is_key_down(KeyCode::D) { stick.x += 1.; }
        if is_key_down(KeyCode::W) { stick.y -= 1.; }
        if is_key_down(KeyCode::S) { stick.y += 1.; }
        if is_key_down(KeyCode::Left) { yaw += dt * 2.; }
        if is_key_down(KeyCode::Right) { yaw -= dt * 2.; }
        if stick.length() > 1. {
            stick = stick.normalize();
        }

        // ---- build mode (2x2x2 blocks in front of you) ----
        let f = vec3(-yaw.sin(), 0., -yaw.cos());
        let r = vec3(-f.z, 0., f.x);
        let c = pl + f * 2.5;
        let bp = vec3(
            (c.x / 2.).round() * 2.,
            ((pl.y - 0.9) / 2.).round() * 2. + 1.,
            (c.z / 2.).round() * 2.,
        );
        if toggle || is_key_pressed(KeyCode::B) {
            build = !build;
        }
        if build && (place || is_key_pressed(KeyCode::E)) && world.len() - base < 300 {
            let cols = [RED, ORANGE, YELLOW, LIME, BLUE, PURPLE];
            let nb = part(bp, vec3(2., 2., 2.), cols[world.len() % 6], Kind::Solid);
            let free = !touching(pl, ph, &nb, 0.) && !world.iter().any(|q| (q.p - bp).length() < 0.1);
            if free {
                world.push(nb);
            }
        }
        if build && (remove || is_key_pressed(KeyCode::Q)) {
            let hit = (base..world.len()).find(|&i| (world[i].p - bp).length() < 0.1);
            if let Some(i) = hit {
                world.remove(i);
            }
        }

        // ---- physics ----
        let mv = (f * -stick.y + r * stick.x) * 6.;
        vel.x = mv.x;
        vel.z = mv.z;
        vel.y -= 25. * dt;
        let mut np = pl;
        np.x += vel.x * dt;
        if blocked(&world, np, ph) { np.x = pl.x; }
        np.z += vel.z * dt;
        if blocked(&world, np, ph) { np.z = pl.z; }
        np.y += vel.y * dt;
        let mut ground = false;
        if blocked(&world, np, ph) {
            ground = vel.y < 0.;
            vel.y = 0.;
            np.y = pl.y;
        }
        pl = np;
        if (jump || is_key_pressed(KeyCode::Space)) && ground {
            vel.y = 9.;
        }

        // ---- lava, checkpoints, win ----
        let dead = pl.y < -30. || world.iter().any(|q| q.k == Kind::Lava && touching(pl, ph, q, 0.05));
        if dead {
            pl = spawn;
            vel = Vec3::ZERO;
        }
        for q in &world {
            if touching(pl, ph, q, 0.1) {
                if q.k == Kind::Check {
                    spawn = q.p + vec3(0., 1.4, 0.);
                }
                if q.k == Kind::Win {
                    win_t = 3.;
                    spawn = start;
                    pl = start;
                    vel = Vec3::ZERO;
                }
            }
        }

        // ---- draw 3D ----
        let dir = vec3(yaw.sin() * pitch.cos(), pitch.sin(), yaw.cos() * pitch.cos());
        clear_background(SKYBLUE);
        set_camera(&Camera3D {
            position: pl + dir * 9.,
            target: pl + vec3(0., 0.5, 0.),
            up: vec3(0., 1., 0.),
            ..Default::default()
        });
        for q in &world {
            draw_cube(q.p, q.s, None, q.c);
            draw_cube_wires(q.p, q.s, BLACK);
        }
        draw_cube(pl + vec3(0., -0.55, 0.), vec3(0.7, 0.7, 0.4), None, GREEN);
        draw_cube(pl + vec3(0., 0.15, 0.), vec3(0.8, 0.7, 0.45), None, BLUE);
        draw_cube(pl + vec3(0., 0.7, 0.), vec3(0.5, 0.4, 0.5), None, YELLOW);
        if build {
            draw_cube_wires(bp, vec3(2., 2., 2.), WHITE);
        }

        // ---- draw HUD ----
        set_default_camera();
        draw_circle(joy.x, joy.y, 100. * u, Color::new(1., 1., 1., 0.2));
        draw_circle(joy.x + stick.x * 60. * u, joy.y + stick.y * 60. * u, 38. * u, Color::new(1., 1., 1., 0.45));
        btn(jump_b, 70. * u, "JUMP");
        btn(build_b, 50. * u, if build { "PLAY" } else { "BUILD" });
        if build {
            btn(plus_b, 50. * u, "+");
            btn(minus_b, 50. * u, "-");
        }
        draw_text("BlockVerse - reach the gold pad!", 20. * u, 40. * u, 36. * u, WHITE);
        if win_t > 0. {
            win_t -= dt;
            draw_text("YOU WIN!", w / 2. - 200. * u, h / 2., 110. * u, GOLD);
        }
        next_frame().await;
    }
                                   }
