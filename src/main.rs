use std::env;

use image::io::Reader as ImageReader;
use palette::color_difference::{HyAb, EuclideanDistance};
use palette::{LinSrgb, Srgb, Luv, IntoColor};

use color_thief::ColorFormat;

use rayon::prelude::*;

fn main() {
    let args: Vec<String> = env::args().collect();

    let filename = &args[1];
    let num: &usize = &args[2].parse().unwrap();

    println!("Filename: {filename}\n");

    let img = ImageReader::open(filename).unwrap().decode().unwrap();
    let img_inter = img.clone().into_rgba8();
    let img_bytes = img_inter.as_raw();

    let now = std::time::Instant::now();
    let colors = color_thief::get_palette(&img_bytes, ColorFormat::Rgba, 5, *num as u8).unwrap();
    println!(
        "{} colors in {} microseconds",
        colors.len(),
        now.elapsed().as_micros()
    );

    for color in colors {
        print!("\x1b[38;2;{};{};{}m██\x1b[0m", color.r, color.g, color.b,);
    }
    println!();

    println!("------");

    let now = std::time::Instant::now();
    let colors = get_palette(
        &img_bytes,
        10,
        num,
        &(img.width() as usize),
    );
    println!(
        "{} colors in {} microseconds",
        colors.len(),
        now.elapsed().as_micros()
    );

    for color in colors {
        print!("\x1b[38;2;{};{};{}m██\x1b[0m", color.r, color.g, color.b,);
    }
    println!();
}

#[derive(Clone)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}

const MIN_DIST: f32 = 30.0;

struct ColorData {
    luv: Luv,
    rgb: [u8; 4],
    position: (usize, usize),
    weight: usize
}

fn get_palette(
    bytes: &[u8],
    step_by: usize,
    target_len: &usize,
    width: &usize,
) -> Vec<Color> {
    // Generate a Vec to store the color data
    let mut colors: Vec<ColorData> = Vec::new();

    // Hold the previous color
    let mut prev_color: Luv = LinSrgb::from_components((0.0, 0.0, 0.0)).into_color();

    // Take chunks of 4 bytes (1 pixel), skipping `step_by` pixels
    'main_img: for (i, pixel) in bytes.chunks_exact(4).enumerate().step_by(step_by) {

        // If the pixel is all black or entirely transparent, skip it
        if (pixel[0] == 0 && pixel[1] == 0 && pixel[2] == 0) || pixel[3] == 0 {
            continue;
        }

        // Create a new `srgb` and `luv` mapped color for the pixel
        let pixel_srgb: LinSrgb<f32> = Srgb::new(pixel[0], pixel[1], pixel[2]).into();
        let pixel_luv: Luv = pixel_srgb.into_color();

        // Get the distance to the previous color, if it's too close, skip it because we don't care
        if pixel_luv.hybrid_distance(prev_color) < 60.0 {
            //prev_color = pixel_luv;
            continue;
        }

        // Get the X and Y position in the image, all we have with `i` is
        // the position within the bytes of the image
        let geo_pos = (i % width, i / width);

        // Loop over all collected colors
        for color in &mut colors {

            // Get the distance to the other color in colorspace `Luv`
            // and then the distance between the pixels on the X Y plane of the image
            let dist = pixel_luv.hybrid_distance(color.luv);
            let geo_dist = geometric_distance(geo_pos, color.position);

            // If the colorspace distance is below MIN_DIST...
            if dist < MIN_DIST {

                // Take the previous pixel's color and the current one
                // and add them together, then divide by 2 (to get a rough average)
                color.luv = (pixel_luv + color.luv) / 2.0;
                color.rgb = average(color.rgb, pixel.try_into().unwrap());

                // Calculate the weight using this math that I just experimented with
                // until it worked
                color.weight += ((100.0 * dist) as f64 + (100.0 * geo_dist.recip())) as usize;

                prev_color = color.luv;
                continue 'main_img; // skips the outer loop
            }
        }

        // If the loop above does not `continue`, then the color wasn't found
        // so add it to the vec!
        colors.push(ColorData {
            luv: pixel_luv,
            rgb: pixel.try_into().unwrap(),
            position: geo_pos,
            weight: 1
        })
    }

    // Sorts by the weight
    colors.sort_by_key(|color| color.weight);
    colors.reverse();

    let mut colors_vec: Vec<Color> = colors
        .iter()
        .map(|color|
        {
            //println!("\x1b[38;2;{};{};{}m██\x1b[0m - {}", x.2[0], x.2[1], x.2[2], x.1);
            Color {
                r: color.rgb[0],
                g: color.rgb[1],
                b: color.rgb[2],
            }
        })
        .collect();

    //let mut colors_vec: Vec<Color> = colors_vec.iter().map(|x| x.clone()).collect();

    if &colors_vec.len() > target_len {
        colors_vec.drain(..target_len).collect()
    } else {
        colors_vec
    }
}

fn average(color1: [u8; 4], color2: [u8; 4]) -> [u8; 4] {
    let mut new_color = [0, 0, 0, 0xFF];
    for i in 0..3 {
        let avg = (((color1[i] as f32 + color2[i] as f32) / 2.0) + 0.5).floor() as u8;
        new_color[i] = avg;
    }

    new_color
}

/*
fn luma_distance(color1: Luv, color2: Luv) -> f32 {
    color1.v
}
*/

fn geometric_distance(pos_1: (usize, usize), pos_2: (usize, usize)) -> f64 {
    let distance_x = pos_1.0.abs_diff(pos_2.0);
    let distance_y = pos_1.1.abs_diff(pos_2.1);
    (distance_y.pow(2) + distance_x.pow(2)) as f64
}
