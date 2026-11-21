//! Generate a synthetic media tree for performance testing.
//!
//! Usage: cargo run --release --example gen_tree -- <root> <item-count>

use std::path::PathBuf;

fn main() {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().expect("usage: gen_tree <root> <count>"));
    let count: usize = args
        .next()
        .expect("usage: gen_tree <root> <count>")
        .parse()
        .expect("item count");

    std::fs::create_dir_all(&root).expect("create root");
    let template = root.join(".template.jpg");
    image::RgbImage::from_fn(64, 48, |x, y| {
        image::Rgb([
            ((x * 4) % 256) as u8,
            ((y * 5) % 256) as u8,
            ((x + y) % 256) as u8,
        ])
    })
    .save(&template)
    .expect("write template");

    let months_per_year = 12usize;
    let total_months = 84usize; // 2020..=2026
    for index in 0..count {
        let slot = index % total_months;
        let year = 2020 + slot / months_per_year;
        let month = slot % months_per_year + 1;
        let directory = root.join(format!("{year}/{month:02}"));
        if index < total_months {
            std::fs::create_dir_all(&directory).expect("create month");
        }
        let destination = directory.join(format!("IMG_{index:06}.jpg"));
        std::fs::hard_link(&template, &destination).expect("link fixture");
    }

    std::fs::remove_file(&template).expect("remove template");
    println!(
        "generated {count} files under {}",
        root.canonicalize().unwrap().display()
    );
}
