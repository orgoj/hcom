//! Window icon drawn in code (TASK-77.94): three agent nodes with CLI
//! colored rings joined by edges, on a dark rounded square.

pub const SIZE: u32 = 64;
const BG: [u8; 3] = [0x1E, 0x29, 0x3B];
const EDGE: [u8; 3] = [0x94, 0xA3, 0xB8];
const NODE_FILL: [u8; 3] = [0xF1, 0xF5, 0xF9];
/// Node centers and ring colors (claude, codex, gemini of the graph).
pub const NODES: [((f32, f32), [u8; 3]); 3] = [
    ((32.0, 16.0), [0xD9, 0x77, 0x06]),
    ((15.0, 46.0), [0x10, 0xA3, 0x7F]),
    ((49.0, 46.0), [0x42, 0x85, 0xF4]),
];

/// Coverage 0..1 of a shape whose signed distance at the pixel is `d`.
fn cover(d: f32) -> f32 {
    (0.5 - d).clamp(0.0, 1.0)
}

fn seg_dist(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (vx, vy) = (b.0 - a.0, b.1 - a.1);
    let t = (((p.0 - a.0) * vx + (p.1 - a.1) * vy) / (vx * vx + vy * vy)).clamp(0.0, 1.0);
    ((p.0 - a.0 - t * vx).powi(2) + (p.1 - a.1 - t * vy).powi(2)).sqrt()
}

/// RGBA pixels, row by row.
pub fn rgba() -> Vec<u8> {
    let s = SIZE as f32;
    let mut out = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let p = (x as f32 + 0.5, y as f32 + 0.5);
            // Rounded square, corner radius 12.
            let (r, h) = (12.0, s / 2.0);
            let (qx, qy) = ((p.0 - h).abs() - (h - r), (p.1 - h).abs() - (h - r));
            let bg_d = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt() + qx.max(qy).min(0.0) - r;
            let alpha = cover(bg_d);
            let mut c = BG.map(f32::from);
            let mut paint = |color: [u8; 3], k: f32| {
                for i in 0..3 {
                    c[i] += (f32::from(color[i]) - c[i]) * k;
                }
            };
            for i in 0..NODES.len() {
                let (a, b) = (NODES[i].0, NODES[(i + 1) % NODES.len()].0);
                paint(EDGE, cover(seg_dist(p, a, b) - 1.5));
            }
            for (center, ring) in NODES {
                let d = ((p.0 - center.0).powi(2) + (p.1 - center.1).powi(2)).sqrt();
                paint(ring, cover(d - 10.0));
                paint(NODE_FILL, cover(d - 6.0));
            }
            out.extend(c.map(|v| v.round() as u8));
            out.push((alpha * 255.0).round() as u8);
        }
    }
    out
}

pub fn icon_data() -> egui::IconData {
    egui::IconData {
        rgba: rgba(),
        width: SIZE,
        height: SIZE,
    }
}
