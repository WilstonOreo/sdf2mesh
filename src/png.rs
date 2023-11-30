// Copyright © Michael Winkelmann <michael@winkelmann.site>
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::io::Write;

pub fn image_data_to_file(
    image_data: Vec<u8>,
    dims: (u32, u32),
    path: impl AsRef<std::path::Path>,
) {
    let mut png_data = Vec::<u8>::with_capacity(image_data.len());
    let mut encoder = png::Encoder::new(std::io::Cursor::new(&mut png_data), dims.0, dims.1);

    encoder.set_color(png::ColorType::Rgba);
    let mut png_writer = encoder.write_header().unwrap();
    png_writer.write_image_data(&image_data[..]).unwrap();
    png_writer.finish().unwrap();
    log::info!("PNG file encoded in memory.");

    let mut file = std::fs::File::create(&path).unwrap();
    file.write_all(&png_data[..]).unwrap();
    log::info!("PNG file written to disc as \"{:?}\".", &path.as_ref());
}

pub trait ToPngFile {
    fn to_png_file(&self, path: impl AsRef<std::path::Path>);
}
