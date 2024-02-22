#![allow(dead_code)]

use std::error::Error;

use image::{
    imageops::resize, DynamicImage, GenericImage, GenericImageView, ImageBuffer, Rgba, RgbaImage,
};

use super::{core::identify, preprocess_image, CharacterIdentifier, Shape};

pub fn get_digit_example(image: &RgbaImage, shape: &Shape) -> RgbaImage {
    let shape_image = shape.create_image(image);
    // let shape_image = resize(&shape_image, 32, 32, image::imageops::FilterType::Nearest);

    shape_image
}

fn diff(a: &Rgba<u8>, b: &Rgba<u8>) -> usize {
    let square = (u8::abs_diff(a.0[0], b.0[0]) as usize).pow(2)
        + (u8::abs_diff(a.0[1], b.0[1]) as usize).pow(2)
        + (u8::abs_diff(a.0[2], b.0[2]) as usize).pow(2)
        + (u8::abs_diff(a.0[3], b.0[3]) as usize).pow(2);

    (square as f64).sqrt() as usize
}

pub fn debug_comparison(
    image: &RgbaImage,
    shape: &Shape,
    digit: &DynamicImage,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, Box<dyn Error>> {
    let shape_image = shape.create_image(&image);

    let b = resize(
        &shape_image,
        digit.width(),
        digit.height(),
        image::imageops::FilterType::Nearest,
    );

    let mut c = ImageBuffer::new(digit.width(), digit.height());

    for (x, y, pixel) in digit.pixels() {
        if pixel.0[3] > 0 && b.get_pixel(x, y).0[3] > 0 {
            c.put_pixel(x, y, Rgba::from([0u8, 255, 0, 255]));
        } else if pixel.0[3] > 0 {
            let a = (diff(&pixel, b.get_pixel(x, y))) as u8;

            c.put_pixel(x, y, Rgba::from([255, 0, 0, a]));
        } else if b.get_pixel(x, y).0[3] > 0 {
            let a = (diff(&pixel, b.get_pixel(x, y)) / 2) as u8;

            c.put_pixel(x, y, Rgba::from([0, 0, 255, a]));
        }
    }

    Ok(c)
}

pub fn main() -> Result<(), Box<dyn Error>> {
    for i in 0..48 {
        let image = image::open(format!("cases/{i}.png"))?.to_rgba8();

        let identifier = CharacterIdentifier::load()?;

        let image = preprocess_image(&image);

        let mut shapes = Shape::find_all(&image)
            .into_iter()
            .filter(|shape| identify::is_digit_shape(shape))
            .collect::<Vec<_>>();

        shapes.sort_by_key(|a| a.bounds().x);

        let mut debug = ImageBuffer::new(image.width(), image.height() * 3);
        debug.copy_from(&image, 0, 0)?;

        for (i, shape) in shapes.iter().enumerate() {
            // shape.create_image(&image).save(format!("shapes/{i}.png"))?;

            let identified = identifier.identify(&shape.create_image(&image));

            println!("{i}: {:?}", identified);

            for &(x, y) in shape.all_pixels() {
                debug.put_pixel(x, y + image.height(), image.get_pixel(x, y).clone());

                if identified.is_some() {
                    debug.put_pixel(x, y + 2 * image.height(), image.get_pixel(x, y).clone());
                }
            }

            // let reference = identifier.debug(&shape.create_image(&image));
            // debug_comparison(&image, &shape, reference)?.save(format!("shapes/diff-{i}.png"))?;
        }

        debug.save(format!("debug/{i}.png"))?;

        // get_digit_example(&preprocessed, &shapes[1]).save("example/9.png")?;
        // get_digit_example(&preprocessed, &shapes[3]).save("example/8.png")?;
        // get_digit_example(&preprocessed, &shapes[4]).save("example/7.png")?;
        // get_digit_example(&preprocessed, &shapes[5]).save("example/6.png")?;
        // get_digit_example(&preprocessed, &shapes[7]).save("example/5.png")?;
        // get_digit_example(&preprocessed, &shapes[9]).save("example/4.png")?;
        // get_digit_example(&preprocessed, &shapes[11]).save("example/3.png")?;
        // get_digit_example(&preprocessed, &shapes[12]).save("example/2.png")?;
        // get_digit_example(&preprocessed, &shapes[13]).save("example/1.png")?;
        // get_digit_example(&preprocessed, &shapes[14]).save("example/0.png")?;
        // get_digit_example(&preprocessed, &shapes[11]).save("example/slash.png")?;

        // debug_comparison(&image, &shapes[1], &image::open("example/slash.png")?)?
        //     .save("compare1.png")?;

        // debug_comparison(&image, &shapes[2], &image::open("example/1.png")?)?
        //     .save("compare2.png")?;

        // debug_comparison(&image, &shapes[2], &image::open("example/slash.png")?)?
        //     .save("compare3.png")?;

        // debug_comparison(&image, &shapes[4], &image::open("example/4.png")?)?
        //     .save("compare4.png")?;

        // debug_comparison(&preprocessed, &shapes[12], &image::open("example/slash.png")?)?
        //     .save("compare2.png")?;

        // debug_comparison(&preprocessed, &shapes[2], &image::open("example/1.png")?)?
        //     .save("compare3.png")?;

        // debug_comparison(&preprocessed, &shapes[2], &image::open("example/3.png")?)?
        //     .save("compare4.png")?;
    }
    Ok(())
}
