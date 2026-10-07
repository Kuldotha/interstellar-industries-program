#![allow(dead_code)]
include!("../../interstellar-industries-client/core/src/lib.rs");
fn main() {
    for n in [5, 8, 12, 16] {
        let planet = generate_surface(n, 0.8, 0.02, 40);
        let alternate = generate_surface(n, 0.8, 0.12, 0);
        let (vertices, triangles) = geodesic(n);
        let mut faces = vec![Vec::new(); vertices.len()];
        let mut centers = vec![V::default(); triangles.len()];
        for i in 0..triangles.len() {
            let f = (i + 12) % triangles.len();
            let ids = triangles[f];
            centers[f] = vertices[ids[0]]
                .add(vertices[ids[1]])
                .add(vertices[ids[2]])
                .scale(1.0 / 3.0);
            for id in ids {
                faces[id].push(f);
            }
        }
        let mut bytes = Vec::new();
        bytes.extend((planet.tiles as u32).to_le_bytes());
        bytes.extend((n as u32).to_le_bytes());
        let mut fixed = bytes.clone();
        let mut expected = Vec::new();
        let mut pentagons = 0;
        let mut permutation=[0u8;256];planet_generation_core::shuffle(&mut permutation,SEED);
        for id in 0..planet.tiles {
            let fs = &faces[id];
            let d = fs
                .iter()
                .fold(V::default(), |a, &f| a.add(centers[f]))
                .scale(1.0 / fs.len() as f32)
                .unit();
            for x in [d.0, d.1, d.2] {
                bytes.extend(x.to_le_bytes());
                fixed.extend(((x * 1048576.0).round() as i32).to_le_bytes());
            }
            let neighbors = &planet.neighbors[id * 6..id * 6 + 6];
            let mut canonical: Vec<_> = neighbors
                .iter()
                .copied()
                .filter(|&x| x != u32::MAX)
                .collect();
            let offset = canonical
                .iter()
                .enumerate()
                .min_by_key(|(_, v)| *v)
                .unwrap()
                .0;
            canonical.rotate_left(offset);
            for j in canonical
                .iter()
                .copied()
                .chain(core::iter::repeat(u32::MAX))
                .take(6)
            {
                fixed.extend(j.to_le_bytes());
            }
            let mut degree = 0;
            for &j in neighbors {
                bytes.extend(j.to_le_bytes());
                if j != u32::MAX {
                    degree += 1;
                    assert!(
                        planet.neighbors[j as usize * 6..j as usize * 6 + 6].contains(&(id as u32))
                    );
                }
            }
            assert!(degree == 5 || degree == 6);
            if degree == 5 {
                pentagons += 1;
            }
            let (surface, height, _, _, _) = blue_tile(d, id,n,&permutation);
            expected.extend([height as u8, surface as u8, planet.features[id] as u8, 0]);
        }
        std::fs::write(
            format!(
                "{}/fixed-topology-{n}.bin",
                std::env::args().nth(1).unwrap()
            ),
            fixed,
        )
        .unwrap();
        assert_eq!(pentagons, 12);
        assert_eq!(planet.tiles, 10 * n * n + 2);
        let differences = planet
            .neighbors
            .chunks(6)
            .zip(alternate.neighbors.chunks(6))
            .filter(|(a, b)| a != b)
            .count();
        for (a, b) in planet
            .neighbors
            .chunks(6)
            .zip(alternate.neighbors.chunks(6))
        {
            let aa: Vec<_> = a.iter().copied().filter(|&i| i != u32::MAX).collect();
            let bb: Vec<_> = b.iter().copied().filter(|&i| i != u32::MAX).collect();
            assert!(
                (0..aa.len())
                    .any(|offset| (0..aa.len()).all(|i| aa[i] == bb[(i + offset) % aa.len()])),
                "not a cyclic rotation"
            );
        }
        println!("resolution {n}: tiles={}, neighbor-order changes with different height scale={differences}",planet.tiles);
        std::fs::write(
            format!("{}/topology-{n}.bin", std::env::args().nth(1).unwrap()),
            bytes,
        )
        .unwrap();
        std::fs::write(
            format!("{}/expected-client-{n}.bin", std::env::args().nth(1).unwrap()),
            expected,
        )
        .unwrap();
    }
}
