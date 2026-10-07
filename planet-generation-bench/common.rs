mod on_demand;
mod noise;
mod noise_fixed;
pub fn hash(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846ca68b);
    x ^ (x >> 16)
}
pub fn tiles(topology: &[u8]) -> usize {
    u32::from_le_bytes(topology[..4].try_into().unwrap()) as usize
}
pub fn state_size(n: usize) -> usize {
    8 + n * 20 + 256
}
fn direction(t: &[u8], i: usize) -> [f32; 3] {
    core::array::from_fn(|j| {
        f32::from_le_bytes(
            t[8 + i * 36 + j * 4..12 + i * 36 + j * 4]
                .try_into()
                .unwrap(),
        )
    })
}
fn adjacent(t: &[u8], i: usize, j: usize) -> usize {
    u32::from_le_bytes(
        t[20 + i * 36 + j * 4..24 + i * 36 + j * 4]
            .try_into()
            .unwrap(),
    ) as usize
}
pub fn terrain(t: &[u8], state: &mut [u8], start: usize, count: usize, mode: u8, seed: u32) {
    let mut perm = [0u8; 256];
    if mode != 0 {
        shuffle(&mut perm, seed);
    }
    terrain_with_permutation(t, state, start, count, mode, &perm);
}
fn shuffle(perm: &mut [u8; 256], seed: u32) {
    for (i, value) in perm.iter_mut().enumerate() {
        *value = i as u8;
    }
    for i in 0..256 {
        perm.swap(i, (hash(seed ^ i as u32) & 255) as usize);
    }
}
pub fn initialize_permutation(state: &mut [u8], n: usize, seed: u32) {
    let offset = 8 + n * 20;
    shuffle((&mut state[offset..offset + 256]).try_into().unwrap(), seed);
    state[..4].copy_from_slice(&seed.to_le_bytes());
    state[4] = 1;
}
pub fn cached_terrain(t: &[u8], state: &mut [u8], start: usize, count: usize) {
    let (body, table) = state.split_at_mut(8 + tiles(t) * 20);
    terrain_with_permutation(t, body, start, count, 4, (&*table).try_into().unwrap());
}
fn terrain_with_permutation(
    t: &[u8],
    state: &mut [u8],
    start: usize,
    count: usize,
    mode: u8,
    perm: &[u8; 256],
) {
    let resolution = u32::from_le_bytes(t[4..8].try_into().unwrap()) as f32;
    let radius = resolution * 5.0 / (2.0 * core::f32::consts::PI);
    for i in start..start + count {
        let [x, y, z] = direction(t, i);
        let height = if mode == 0 {
            let e = libm::sinf(x * 3.1 + z * 1.8 + 0.7) * 0.48
                + libm::sinf(z * 4.0 - y * 2.3 - 0.3) * 0.31
                + libm::sinf(x * 7.0 + y * 5.0 + z * 3.0) * 0.13;
            if e < 0.04 {
                0
            } else if e < 0.16 {
                1
            } else if e > 0.64 {
                3
            } else if e > 0.42 {
                2
            } else {
                1
            }
        } else if mode == 4 {
            noise_fixed::height([x, y, z], 0.1 * radius, &perm)
        } else {
            let h = ((noise::octave([x, y, z], 0.1 * radius, &perm, true) + 1.0) * 3.0 - 3.0 + 1.2)
                .clamp(-3.0, 3.0);
            libm::rintf(h) as i8
        };
        let surface = if height <= 0 {
            0
        } else if y.abs() > 0.82 {
            2
        } else {
            1
        };
        let stone = hash(i as u32 ^ 0x97531) & 65535;
        let offset = 8 + i * 4;
        state[offset] = height as u8;
        state[offset + 1] = surface;
        state[offset + 2] = if stone <= 15073 { 2 } else { 0 };
        state[offset + 3] = 0;
        if mode == 2 {
            let humidity = noise::octave(
                [x * radius, y * radius, z * radius],
                resolution * 0.01,
                &perm,
                false,
            );
            let temperature = noise::octave(
                [x * radius, y * radius, z * radius],
                resolution * 0.06,
                &perm,
                false,
            );
            state[offset + 2] = ((humidity + 1.0) * 100.0) as u8;
            state[offset + 3] = ((temperature + 1.0) * 100.0) as u8;
        }
    }
}
pub fn forests(t: &[u8], state: &mut [u8], seed: u32) {
    let n = tiles(t);
    let resolution = u32::from_le_bytes(t[4..8].try_into().unwrap()) as usize;
    let (_, body) = state.split_at_mut(8);
    let (records, scratch) = body.split_at_mut(n * 4);
    let words =
        unsafe { core::slice::from_raw_parts_mut(scratch.as_mut_ptr().cast::<u32>(), n * 4) };
    let (seeds, rest) = words.split_at_mut(n);
    let (frontier, rest) = rest.split_at_mut(n);
    let (seen, group) = rest.split_at_mut(n);
    seen.fill(0);
    let eligible = |id: usize, rec: &[u8]| rec[id * 4 + 1] == 1 && rec[id * 4 + 2] & 2 == 0;
    let mut len = 0;
    for i in 0..n {
        if eligible(i, records) {
            seeds[len] = i as u32;
            len += 1;
        }
    }
    let budget = len * 30 / 100;
    seeds[..len].sort_unstable_by_key(|&i| hash(i ^ seed ^ 771));
    let limit = (12 * resolution * resolution / 64).max(6);
    let mut placed = 0;
    for &origin in &seeds[..len] {
        let origin = origin as usize;
        if records[origin * 4 + 3] != 0 || records[origin * 4 + 2] & 1 != 0 || budget - placed < 6 {
            continue;
        }
        let max = limit.min(budget - placed);
        let mut pending = 1;
        let mut size = 0;
        frontier[0] = origin as u32;
        let mark = origin as u32 + 1;
        seen[origin] = mark;
        while pending > 0 && size < max {
            frontier[..pending].sort_unstable_by_key(|&i| hash(i ^ origin as u32 ^ seed));
            pending -= 1;
            let id = frontier[pending] as usize;
            group[size] = id as u32;
            size += 1;
            for side in 0..6 {
                let other = adjacent(t, id, side);
                if other >= n {
                    continue;
                }
                if eligible(other, records)
                    && records[other * 4 + 3] == 0
                    && records[other * 4 + 2] & 1 == 0
                    && seen[other] != mark
                {
                    seen[other] = mark;
                    frontier[pending] = other as u32;
                    pending += 1;
                }
            }
        }
        if size < 6 {
            continue;
        }
        placed += size;
        for &id in &group[..size] {
            records[id as usize * 4 + 2] |= 1;
        }
        for &id in &group[..size] {
            records[id as usize * 4 + 3] = 1;
            for side in 0..6 {
                let other = adjacent(t, id as usize, side);
                if other < n {
                    records[other * 4 + 3] = 1;
                }
            }
        }
    }
}
