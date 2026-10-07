#![no_std]
pub mod noise_fixed;
use noise_fixed::Q;
pub const VERSION: u8 = 3;
pub const STATE_SIZE: usize = 272;
pub fn initialize(state: &mut [u8], seed: u32) {
    state[..4].copy_from_slice(&seed.to_le_bytes());
    state[4] = VERSION;
    crate::shuffle((&mut state[8..264]).try_into().unwrap(), seed);
}
pub fn tile(t: &[u8], id: usize, p: &[u8; 256], seed: u32) -> [u8; 3] {
    let d: [i64; 3] = core::array::from_fn(|j| {
        i32::from_le_bytes(
            t[8 + id * 36 + j * 4..12 + id * 36 + j * 4]
                .try_into()
                .unwrap(),
        ) as i64
    });
    sample(d,u32::from_le_bytes(t[4..8].try_into().unwrap()),id as u32,p,seed)
}
pub fn sample(d:[i64;3],resolution:u32,id:u32,p:&[u8;256],seed:u32)->[u8;3] {
    let resolution=resolution as i64;
    let height =
        noise_fixed::height_with_bias::<4>(d.map(|x| x * resolution * 83443 / Q), p, 153 * Q / 100);
    let surface = if height <= 0 {
        0
    } else if d[1].abs() > Q * 82 / 100 {
        2
    } else {
        1
    };
    let stone = hash(seed ^ hash(id as u32) ^ 0x97531) & 65535 <= 15073;
    let offset: [i64; 3] =
        core::array::from_fn(|j| (hash(seed ^ (0xa113_u32 + j as u32)) & 65535) as i64 * Q / 4096);
    let mut v = core::array::from_fn(|j| d[j] * 3 / 2 + offset[j]);
    let mut forest = 0;
    for weight in [4, 1] {
        forest += noise_fixed::raw(v, p) * weight;
        v = v.map(|x| x * 2);
    }
    let forest = surface == 1 && !stone && forest > Q / 2;
    [
        height as u8,
        surface,
        if stone {
            2
        } else if forest {
            1
        } else {
            0
        },
    ]
}
pub fn query(t: &[u8], state: &mut [u8], id: usize, side: Option<usize>) {
    let seed = u32::from_le_bytes(state[..4].try_into().unwrap());
    let p = (&state[8..264]).try_into().unwrap();
    let own = tile(t, id, p, seed);
    let mut result = [0u8; 8];
    result[..3].copy_from_slice(&own);
    let mut valid = own[1] != 0;
    if let Some(side) = side {
        let next = adjacent(t, id, side);
        if next >= crate::tiles(t) {
            valid = false;
            result[3] = 255;
        } else {
            let other = tile(t, next, p, seed);
            result[3] = other[0];
            valid &= other[1] == 0;
        }
    }
    result[4] = u8::from(valid);
    state[264..272].copy_from_slice(&result);
}

pub fn hash(mut x:u32)->u32 {x^=x>>16;x=x.wrapping_mul(0x7feb352d);x^=x>>15;x=x.wrapping_mul(0x846ca68b);x^(x>>16)}
pub fn shuffle(perm:&mut[u8;256],seed:u32){for(i,v)in perm.iter_mut().enumerate(){*v=i as u8;}for i in 0..256{perm.swap(i,(hash(seed^i as u32)&255)as usize);}}
pub fn tiles(t:&[u8])->usize{u32::from_le_bytes(t[..4].try_into().unwrap())as usize}
fn adjacent(t:&[u8],i:usize,j:usize)->usize{u32::from_le_bytes(t[20+i*36+j*4..24+i*36+j*4].try_into().unwrap())as usize}
