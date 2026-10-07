pub const Q: i64 = 1 << 20;
fn div(a: i64, b: u64) -> i64 {
    if a < 0 {
        -(((-a) as u64 / b) as i64)
    } else {
        (a as u64 / b) as i64
    }
}
fn mul(a: i64, b: i64) -> i64 {
    div(a * b, Q as u64)
}
fn floor(x: i64) -> i64 {
    if x > 0 {
        div(x, Q as u64)
    } else {
        x / Q - 1
    }
}
fn gradient(h: u8, x: i64, y: i64, z: i64) -> i64 {
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
pub fn raw([x, y, z]: [i64; 3], p: &[u8; 256]) -> i64 {
    let s = div(x + y + z, 3);
    let i = floor(x + s);
    let j = floor(y + s);
    let k = floor(z + s);
    let t = div((i + j + k) * Q, 6);
    let x = x - (i * Q - t);
    let y = y - (j * Q - t);
    let z = z - (k * Q - t);
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
    let mut value = 0;
    for (corner, offset) in [[0, 0, 0], a, b, [1, 1, 1]].iter().enumerate() {
        let u = x - offset[0] * Q + div(corner as i64 * Q, 6);
        let v = y - offset[1] * Q + corner as i64 * Q / 6;
        let w = z - offset[2] * Q + corner as i64 * Q / 6;
        let q = 3 * Q / 5 - mul(u, u) - mul(v, v) - mul(w, w);
        if q < 0 {
            continue;
        }
        let g = p[((i
            + offset[0]
            + p[((j + offset[1] + p[((k + offset[2]) & 255) as usize] as i64) & 255) as usize]
                as i64)
            & 255) as usize]
            % 12;
        let q = mul(q, q);
        value += mul(mul(q, q), gradient(g, u, v, w));
    }
    32 * value
}
pub fn height(d: [f32; 3], frequency: f32, p: &[u8; 256]) -> i8 {
    height_fixed(d.map(|v| (v * frequency * Q as f32) as i64), p)
}
pub fn height_fixed(d: [i64; 3], p: &[u8; 256]) -> i8 {
    height_with_bias::<8>(d, p, 6 * Q / 5)
}
pub fn height_with_bias<const OCTAVES: usize>(mut d: [i64; 3], p: &[u8; 256], bias: i64) -> i8 {
    let (mut value, mut amplitude, mut total) = (0, Q, 0);
    for _ in 0..OCTAVES {
        value += mul(raw(d, p).abs(), amplitude);
        total += amplitude;
        amplitude /= 2;
        for x in &mut d {
            *x *= 2;
        }
    }
    let h = ((div(value * Q, total as u64) * 2 - Q) * 3 + bias).clamp(-3 * Q, 3 * Q);
    let base = h.div_euclid(Q);
    let remainder = h.rem_euclid(Q);
    (base + i64::from(remainder > Q / 2 || remainder == Q / 2 && base % 2 != 0)) as i8
}
