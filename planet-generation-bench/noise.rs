fn floor(x: f32) -> i32 {
    if x > 0.0 {
        x as i32
    } else {
        x as i32 - 1
    }
}
fn gradient(h: u8, x: f32, y: f32, z: f32) -> f32 {
    let h = h & 15;
    let u = if h < 8 { x } else { y };
    let v = if h < 4 {
        y
    } else if h == 12 || h == 14 {
        x
    } else {
        z
    };
    (if h & 1 == 0 { u } else { -u }) + (if h & 2 == 0 { v } else { -v })
}
fn raw([x, y, z]: [f32; 3], p: &[u8; 256]) -> f32 {
    let s = (x + y + z) * (1.0 / 3.0);
    let i = floor(x + s);
    let j = floor(y + s);
    let k = floor(z + s);
    let t = (i + j + k) as f32 * (1.0 / 6.0);
    let x = x - (i as f32 - t);
    let y = y - (j as f32 - t);
    let z = z - (k as f32 - t);
    let (a, b) = if x >= y {
        if y >= z {
            ([1, 0, 0], [1, 1, 0])
        } else if x >= z {
            ([1, 0, 0], [1, 0, 1])
        } else {
            ([0, 0, 1], [1, 0, 1])
        }
    } else if y < z {
        ([0, 0, 1], [0, 1, 1])
    } else if x < z {
        ([0, 1, 0], [0, 1, 1])
    } else {
        ([0, 1, 0], [1, 1, 0])
    };
    let mut value = 0.0;
    for (corner, offset) in [[0, 0, 0], a, b, [1, 1, 1]].iter().enumerate() {
        let u = x - offset[0] as f32 + corner as f32 * (1.0 / 6.0);
        let v = y - offset[1] as f32 + corner as f32 * (1.0 / 6.0);
        let w = z - offset[2] as f32 + corner as f32 * (1.0 / 6.0);
        let q = 0.6 - u * u - v * v - w * w;
        if q < 0.0 {
            continue;
        }
        let g = p[((i
            + offset[0]
            + p[((j + offset[1] + p[((k + offset[2]) & 255) as usize] as i32) & 255) as usize]
                as i32)
            & 255) as usize]
            % 12;
        let q = q * q;
        value += q * q * gradient(g, u, v, w);
    }
    32.0 * value
}
pub fn octave(mut d: [f32; 3], frequency: f32, p: &[u8; 256], billow: bool) -> f32 {
    for x in &mut d {
        *x *= frequency;
    }
    let (mut value, mut amplitude, mut total) = (0.0, 1.0, 0.0);
    for _ in 0..8 {
        let sample = raw(d, p);
        value += (if billow { sample.abs() } else { sample }) * amplitude;
        total += amplitude;
        amplitude *= 0.5;
        for x in &mut d {
            *x *= 2.0;
        }
    }
    if billow {
        value / total * 2.0 - 1.0
    } else {
        value / total
    }
}
