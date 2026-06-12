use std::fs;
use std::io::BufWriter;
use std::path::Path;

fn main() {
    // Ensure assets directory exists
    let assets = Path::new("assets");
    fs::create_dir_all(assets).unwrap();

    // Generate placeholder PNGs for each cat state
    let states = [
        ("cat_idle", 0xFF, 0x9F, 0x4A),
        ("cat_remind", 0xFF, 0xA5, 0x4A),
        ("cat_warning", 0xF5, 0xA6, 0x23),
        ("cat_sad", 0xE8, 0xC9, 0xA0),
        ("cat_recover", 0xFF, 0xB0, 0x60),
        ("cat_happy", 0xFF, 0xB3, 0x47),
        ("cat_sleeping", 0xF0, 0xB8, 0x7A),
    ];

    for (name, r, g, b) in &states {
        let path = assets.join(format!("{}.png", name));
        if path.exists() {
            continue; // Don't overwrite user's real images
        }
        generate_png(&path, 120, 160, *r, *g, *b);
        println!("cargo:warning=Generated placeholder: {}", path.display());
    }

    // Compile Slint UI
    slint_build::compile("ui/pet.slint").unwrap();
}

fn generate_png(path: &Path, width: u32, height: u32, r: u8, g: u8, b: u8) {
    let file = fs::File::create(path).unwrap();
    let w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap();

    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for _ in 0..height {
        for _ in 0..width {
            data.push(r);
            data.push(g);
            data.push(b);
            data.push(255); // alpha
        }
    }

    writer.write_image_data(&data).unwrap();
}
