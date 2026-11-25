//! Prépare les illustrations: supprime les points parasites du détourage,
//! recadre, redimensionne (alpha prémultiplié) et écrit des PNG dans assets/.
use image::{imageops, ImageBuffer, Rgba, RgbaImage};
use std::path::{Path, PathBuf};

type Img = RgbaImage;

fn load(p: &Path) -> Img {
    image::open(p).unwrap_or_else(|e| panic!("{}: {e}", p.display())).to_rgba8()
}

/// Supprime les composantes de pixels visibles minuscules (poussière du détourage).
fn despeckle(img: &mut Img, min_frac: f32) {
    let (w, h) = (img.width() as i32, img.height() as i32);
    let idx = |x: i32, y: i32| (y * w + x) as usize;
    let vis: Vec<bool> = img.pixels().map(|p| p[3] > 24).collect();
    // Dilatation (rayon 5) pour relier les morceaux proches d'un même objet.
    let r = 5;
    let mut dil = vec![false; vis.len()];
    for y in 0..h {
        for x in 0..w {
            if vis[idx(x, y)] {
                for dy in -r..=r {
                    for dx in -r..=r {
                        let (nx, ny) = (x + dx, y + dy);
                        if nx >= 0 && ny >= 0 && nx < w && ny < h {
                            dil[idx(nx, ny)] = true;
                        }
                    }
                }
            }
        }
    }
    let mut label = vec![0u32; vis.len()];
    let mut counts: Vec<usize> = vec![0];
    let mut next = 1u32;
    for sy in 0..h {
        for sx in 0..w {
            if !dil[idx(sx, sy)] || label[idx(sx, sy)] != 0 {
                continue;
            }
            let mut stack = vec![(sx, sy)];
            label[idx(sx, sy)] = next;
            let mut n = 0usize;
            while let Some((x, y)) = stack.pop() {
                if vis[idx(x, y)] {
                    n += 1;
                }
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx >= 0 && ny >= 0 && nx < w && ny < h && dil[idx(nx, ny)] && label[idx(nx, ny)] == 0 {
                        label[idx(nx, ny)] = next;
                        stack.push((nx, ny));
                    }
                }
            }
            counts.push(n);
            next += 1;
        }
    }
    let max = *counts.iter().max().unwrap_or(&1) as f32;
    for y in 0..h {
        for x in 0..w {
            let l = label[idx(x, y)] as usize;
            let p = img.get_pixel_mut(x as u32, y as u32);
            if l == 0 || (counts[l] as f32) < max * min_frac || p[3] < 10 {
                *p = Rgba([0, 0, 0, 0]);
            }
        }
    }
}

fn bbox(img: &Img) -> Option<(u32, u32, u32, u32)> {
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
    for (x, y, p) in img.enumerate_pixels() {
        if p[3] > 8 {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
    }
    (x0 != u32::MAX).then_some((x0, y0, x1 + 1, y1 + 1))
}

fn crop(img: &Img, b: (u32, u32, u32, u32)) -> Img {
    imageops::crop_imm(img, b.0, b.1, b.2 - b.0, b.3 - b.1).to_image()
}

/// Redimensionne en alpha prémultiplié pour éviter les liserés.
fn resize(img: &Img, scale: f32) -> Img {
    let (w, h) = (((img.width() as f32 * scale) as u32).max(1), ((img.height() as f32 * scale) as u32).max(1));
    let pm: ImageBuffer<Rgba<f32>, Vec<f32>> = ImageBuffer::from_fn(img.width(), img.height(), |x, y| {
        let p = img.get_pixel(x, y);
        let a = p[3] as f32 / 255.0;
        Rgba([p[0] as f32 / 255.0 * a, p[1] as f32 / 255.0 * a, p[2] as f32 / 255.0 * a, a])
    });
    let r = imageops::resize(&pm, w, h, imageops::FilterType::Lanczos3);
    ImageBuffer::from_fn(w, h, |x, y| {
        let p = r.get_pixel(x, y);
        let a = p[3].clamp(0.0, 1.0);
        if a < 0.004 {
            Rgba([0, 0, 0, 0])
        } else {
            let c = |v: f32| ((v / a).clamp(0.0, 1.0) * 255.0).round() as u8;
            Rgba([c(p[0]), c(p[1]), c(p[2]), (a * 255.0).round() as u8])
        }
    })
}

fn save(img: &Img, out: &Path, name: &str) {
    let p = out.join(name);
    img.save(&p).unwrap();
    println!("{name}: {}x{}", img.width(), img.height());
}

/// Découpe une planche en éléments séparés (ordre de lecture), chacun recadré.
fn split(img: &Img, r: i32, min_frac: f32, row_tol: f32) -> Vec<(Img, (u32, u32, u32, u32))> {
    let (w, h) = (img.width() as i32, img.height() as i32);
    let idx = |x: i32, y: i32| (y * w + x) as usize;
    let vis: Vec<bool> = img.pixels().map(|p| p[3] > 24).collect();
    let mut dil = vec![false; vis.len()];
    for y in 0..h {
        for x in 0..w {
            if vis[idx(x, y)] {
                for dy in -r..=r {
                    for dx in -r..=r {
                        let (nx, ny) = (x + dx, y + dy);
                        if nx >= 0 && ny >= 0 && nx < w && ny < h {
                            dil[idx(nx, ny)] = true;
                        }
                    }
                }
            }
        }
    }
    let mut label = vec![0u32; vis.len()];
    let mut counts = vec![0usize];
    let mut next = 1u32;
    for sy in 0..h {
        for sx in 0..w {
            if !dil[idx(sx, sy)] || label[idx(sx, sy)] != 0 {
                continue;
            }
            let mut stack = vec![(sx, sy)];
            label[idx(sx, sy)] = next;
            let mut n = 0usize;
            while let Some((x, y)) = stack.pop() {
                if vis[idx(x, y)] {
                    n += 1;
                }
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx >= 0 && ny >= 0 && nx < w && ny < h && dil[idx(nx, ny)] && label[idx(nx, ny)] == 0 {
                        label[idx(nx, ny)] = next;
                        stack.push((nx, ny));
                    }
                }
            }
            counts.push(n);
            next += 1;
        }
    }
    let max = *counts.iter().max().unwrap_or(&1) as f32;
    let mut comps: Vec<(f32, f32, (Img, (u32, u32, u32, u32)))> = Vec::new();
    for l in 1..counts.len() {
        if (counts[l] as f32) < max * min_frac {
            continue;
        }
        let mut c = Img::new(img.width(), img.height());
        for y in 0..h {
            for x in 0..w {
                if label[idx(x, y)] as usize == l && vis[idx(x, y)] {
                    c.put_pixel(x as u32, y as u32, *img.get_pixel(x as u32, y as u32));
                }
            }
        }
        if let Some(b) = bbox(&c) {
            comps.push(((b.1 + b.3) as f32 / 2.0, (b.0 + b.2) as f32 / 2.0, (crop(&c, b), b)));
        }
    }
    // Ordre de lecture: lignes (écart vertical > 220 px = nouvelle ligne), puis de gauche à droite.
    comps.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let mut rows: Vec<Vec<(f32, f32, (Img, (u32, u32, u32, u32)))>> = Vec::new();
    for c in comps {
        match rows.last_mut() {
            Some(row) if (c.0 - row[0].0).abs() < row_tol => row.push(c),
            _ => rows.push(vec![c]),
        }
    }
    let mut out = Vec::new();
    for mut row in rows {
        row.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        out.extend(row.into_iter().map(|c| c.2));
    }
    out
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let refdir = PathBuf::from(&args[1]);
    let out = PathBuf::from(&args[2]);
    std::fs::create_dir_all(&out).unwrap();

    // Rennes: 4 images au même cadrage (union des boîtes) et à la même échelle.
    let mut deer: Vec<Img> = (1..=4)
        .map(|i| {
            let mut im = load(&refdir.join(format!("rennes_movements ({i}).png")));
            despeckle(&mut im, 0.05);
            im
        })
        .collect();
    let mut u = (u32::MAX, u32::MAX, 0u32, 0u32);
    for im in &deer {
        let b = bbox(im).unwrap();
        u = (u.0.min(b.0), u.1.min(b.1), u.2.max(b.2), u.3.max(b.3));
    }
    println!("union rennes: {:?}", u);
    let scale = 330.0 / (u.2 - u.0) as f32;
    for (i, im) in deer.iter_mut().enumerate() {
        let c = resize(&crop(im, u), scale);
        save(&c, &out, &format!("reindeer_{}.png", i + 1));
    }

    // Traîneau: on retire les rennes (x >= 860) et le début des rênes (bande au-dessus du patin).
    let mut sl = load(&refdir.join("traineau_rennes.png"));
    despeckle(&mut sl, 0.05);
    for y in 0..sl.height() {
        for x in 0..sl.width() {
            if x >= 860 || (x >= 706 && y <= 468) {
                sl.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            }
        }
    }
    let b = bbox(&sl).unwrap();
    println!("traineau bbox (px d'origine): {:?}", b);
    save(&resize(&crop(&sl, b), 0.62), &out, "sleigh.png");

    // Sapins et guirlande: composantes séparées par la position.
    let mut tg = load(&refdir.join("Festive Christmas Trees and Garland.png"));
    despeckle(&mut tg, 0.02);
    let top = |x0: u32, x1: u32, y1: u32| {
        let mut im = tg.clone();
        for y in 0..im.height() {
            for x in 0..im.width() {
                if !(x >= x0 && x < x1 && y < y1) {
                    im.put_pixel(x, y, Rgba([0, 0, 0, 0]));
                }
            }
        }
        im
    };
    let bands = [(0u32, 640u32), (640, 1210), (1210, 1774)];
    for (i, (x0, x1)) in bands.iter().enumerate() {
        let im = top(*x0, *x1, 690);
        let b = bbox(&im).unwrap();
        save(&resize(&crop(&im, b), 0.5), &out, &format!("tree_{}.png", i + 1));
    }
    // Guirlande: nouvelle image dédiée (nœuds aux extrémités et au centre).
    let mut g = load(&refdir.join("Festive Pine Garland with Red Bow.png"));
    despeckle(&mut g, 0.02);
    let gb = bbox(&g).unwrap();
    save(&resize(&crop(&g, gb), 0.4), &out, "garland.png");

    // Planches de personnages et d'objets: éléments séparés.
    let sheets: [(&str, &str, f32); 4] = [
        ("Cheerful Winter Character Parade-1.png", "ch_p", 0.5),
        ("Cheerful Winter Holiday Sticker Set-2.png", "ch_s", 0.5),
        ("Festive Christmas Icon Collection-2.png", "ic_", 0.4),
        ("Festive Christmas Wreath and Gift Set-1.png", "wr_", 0.4),
    ];
    for (file, prefix, k) in sheets {
        let mut im = load(&refdir.join(file));
        despeckle(&mut im, 0.03);
        let parts: Vec<Img> = split(&im, 10, 0.03, 220.0).into_iter().map(|p| p.0).collect();
        println!("{file}: {} éléments", parts.len());
        for (i, part) in parts.iter().enumerate() {
            save(&resize(part, k), &out, &format!("{prefix}{}.png", i + 1));
        }
    }

    // Père Noël: 8 images de marche (rangée du bas), alignées sur un même cadrage.
    let mut sa = load(&refdir.join("Santa Claus Animation Asset Sheet.png"));
    despeckle(&mut sa, 0.002);
    let mut strip = sa.clone();
    for y in 0..strip.height() {
        for x in 0..strip.width() {
            if y < 960 {
                strip.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            }
        }
    }
    let frames = split(&strip, 3, 0.2, 400.0);
    println!("père Noël marche: {} images", frames.len());
    for (i, (_, b)) in frames.iter().enumerate() {
        println!("  frame {i}: bbox {:?} (x0 - i*cell = {})", b, b.0 as f32 - i as f32 * (1254.0 / 8.0));
    }
    // Cadre commun: largeur max, alignement sur le bas (sol) et sur le centre de la tête (x du chapeau).
    let wmax = frames.iter().map(|(_, b)| b.2 - b.0).max().unwrap_or(1);
    let hmax = frames.iter().map(|(_, b)| b.3 - b.1).max().unwrap_or(1);
    let base_y = frames.iter().map(|(_, b)| b.3).max().unwrap_or(1);
    for (i, (im, b)) in frames.iter().enumerate() {
        let mut canvas = Img::new(wmax + 40, hmax + 20);
        // Aligné par le bas: les pieds au sol, légèrement relevés quand ils se lèvent.
        let oy = (b.1 as i32 - (base_y as i32 - hmax as i32)).max(0) as u32;
        let ox = 20 + (wmax - (b.2 - b.0)) / 2;
        imageops::overlay(&mut canvas, im, ox as i64, oy as i64);
        save(&resize(&canvas, 0.8), &out, &format!("santa_walk_{}.png", i + 1));
    }

    // Père Noël qui salue (en haut à gauche): on garde la composante la plus à gauche.
    let mut wave = sa.clone();
    for y in 0..wave.height() {
        for x in 0..wave.width() {
            if x > 420 || y > 375 {
                wave.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            }
        }
    }
    let wp = split(&wave, 3, 0.2, 400.0);
    println!("salut: {} composantes", wp.len());
    if let Some((im, _)) = wp.first() {
        save(&resize(im, 0.8), &out, "santa_wave.png");
    }

    // Collection d'icônes (4 rangées): éléments séparés dans l'ordre de lecture.
    let mut ic = load(&refdir.join("Festive Christmas Icon Collection.png"));
    despeckle(&mut ic, 0.002);
    let parts = split(&ic, 8, 0.002, 100.0);
    println!("collection d'icônes: {} éléments", parts.len());
    for (i, (im, b)) in parts.iter().enumerate() {
        println!("  icon {:02}: bbox {:?}", i + 1, b);
        save(&resize(im, 0.8), &out, &format!("ico_{:02}.png", i + 1));
    }
}
