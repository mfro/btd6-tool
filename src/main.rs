use image::{
    imageops::{crop_imm, resize},
    DynamicImage, GenericImageView, ImageBuffer, Rgba, RgbaImage,
};
use std::{
    cmp::Ordering,
    error::Error,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
use windows_capture::{
    capture::WindowsCaptureHandler, frame::Frame, graphics_capture_api::InternalCaptureControl,
};

mod identify;
mod shape;
mod sync;
pub use identify::CharacterIdentifier;
pub use shape::Shape;
pub use sync::ShareThing;
struct State {
    frame: ShareThing<RgbaImage>,
}

struct Capture {
    state: Arc<State>,
}

impl WindowsCaptureHandler for Capture {
    type Flags = Arc<State>;
    type Error = Box<dyn Error + Send + Sync>;

    fn new(state: Self::Flags) -> Result<Self, Self::Error> {
        Ok(Capture { state })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        let width = frame.width();
        let height = frame.height();

        let mut buffer = frame.buffer()?;
        let raw_buffer = buffer.as_raw_nopadding_buffer()?;

        let image: RgbaImage = ImageBuffer::from_raw(width, height, raw_buffer.to_vec()).unwrap();

        self.state.frame.put(image);

        Ok(())
    }
}

#[derive(Default, Clone, Debug, PartialEq, Eq, Hash)]
struct IdentifiedCharacter {
    shape: Shape,
    character: char,
}

fn preprocess_image(image: &RgbaImage) -> RgbaImage {
    let image = crop_imm(image, 100, 50, 1220, 40).to_image();

    let t0 = Instant::now();
    let mut out = ImageBuffer::new(image.width(), image.height());

    for (x, y, pixel) in image.enumerate_pixels() {
        let r = pixel.0[0] as usize;
        let g = pixel.0[1] as usize;
        let b = pixel.0[2] as usize;

        // let err = (2 * r).abs_diff(b + g) + (2 * b).abs_diff(r + g) + (2 * g).abs_diff(r + b);

        // if err <= 64 {
        // out.put_pixel(x, y, pixel.clone());
        // }

        if r + g + b >= 256 {
            out.put_pixel(x, y, pixel.clone());
        }
    }

    let t1 = Instant::now();
    println!("filter greyscale {:?}", t1 - t0);

    out
}

fn is_digit_shape(shape: &Shape) -> bool {
    (128..1024).contains(&shape.len())
}

fn identify_digits(
    identifier: &CharacterIdentifier,
    image: &RgbaImage,
) -> Result<Vec<IdentifiedCharacter>, Box<dyn Error>> {
    let t0 = Instant::now();

    let shapes: Vec<_> = Shape::find_all(image)
        .into_iter()
        .filter(|shape| is_digit_shape(shape))
        .collect();

    let t1 = Instant::now();
    println!("find shapes {:?}", t1 - t0);

    let t0 = Instant::now();

    let identified = shapes
        .iter()
        .filter_map(|shape| {
            let shape_image = shape.create_image(image);
            let (character, score) = identifier.identify(&shape_image);

            (score >= 700000).then(|| IdentifiedCharacter {
                character,
                shape: shape.clone(),
            })
        })
        .collect();

    let t1 = Instant::now();
    println!("identify digits {:?}", t1 - t0);

    // let resized_shape_images: Vec<_> = shape_images
    //     .iter()
    //     .map(|image| resize(image, 32, 32, FilterType::Nearest))
    //     .collect();

    // resized_shape_images[0].save("digits/1.png")?;
    // resized_shape_images[1].save("digits/2.png")?;
    // resized_shape_images[2].save("digits/3.png")?;
    // resized_shape_images[3].save("digits/4.png")?;
    // resized_shape_images[4].save("digits/5.png")?;
    // resized_shape_images[5].save("digits/6.png")?;
    // resized_shape_images[6].save("digits/7.png")?;
    // resized_shape_images[7].save("digits/8.png")?;
    // resized_shape_images[8].save("digits/9.png")?;
    // resized_shape_images[9].save("digits/0.png")?;

    // for (i, shape_image) in shape_images.iter().enumerate() {
    //     shape_image.save(format!("shapes/{i}.png"))?;

    //     let digit = identifier.identify(shape_image);

    //     println!("{:?}", digit);
    // }

    // for shape in shapes.iter() {
    //     for &(x, y) in shape.all_pixels() {
    //         step2.put_pixel(x, y, image.get_pixel(x, y).clone());
    //     }
    // }

    // let t1 = Instant::now();
    // println!("filter shape size {:?}", t1 - t0);

    // Ok(step2)

    Ok(identified)
}

fn is_line_break(a: &IdentifiedCharacter, b: &IdentifiedCharacter) -> bool {
    let a = a.shape.bounds();
    let b = b.shape.bounds();

    (b.y).abs_diff(a.y) >= b.height
}

fn is_word_break(a: &IdentifiedCharacter, b: &IdentifiedCharacter) -> bool {
    let a = a.shape.bounds();
    let b = b.shape.bounds();

    (b.x).abs_diff(a.x) >= b.width * 3
}

fn group_words(src: impl Iterator<Item = IdentifiedCharacter>) -> Vec<Vec<IdentifiedCharacter>> {
    let mut words = vec![];
    let mut previous: Option<IdentifiedCharacter> = None;

    for ch in src {
        let next_word = match previous {
            Some(previous) => is_line_break(&previous, &ch) || is_word_break(&previous, &ch),
            None => true,
        };

        previous = Some(ch.clone());

        if next_word {
            words.push(vec![]);
        }

        match words.last_mut() {
            Some(word) => word.push(ch),
            None => words.push(vec![ch]),
        }
    }

    words
}

fn get_digit_example(image: &RgbaImage, shape: &Shape) -> RgbaImage {
    let shape_image = shape.create_image(image);
    // let shape_image = resize(&shape_image, 32, 32, image::imageops::FilterType::Nearest);

    shape_image
}

fn main() -> Result<(), Box<dyn Error>> {
    let image = image::open("0.png")?;
    let identifier = CharacterIdentifier::load()?;

    process_image(&identifier, &image.into_rgba8())?;

    // let x = image::open("digits/0.png")?;
    // let shapes = Shape::find_all(&x);

    // let path = identify::contour(&shapes[0]);
    // println!("{:?}", path);

    // let state = Arc::new(State {
    //     frame: Default::default(),
    // });

    // {
    //     let state = state.clone();

    //     thread::spawn(move || {
    //         process_thread(&state.frame).unwrap();
    //     });
    // }

    // let x = Window::from_name("BloonsTD6-Epic").unwrap();

    // Capture::start(Settings {
    //     item: x.try_into().unwrap(),
    //     capture_cursor: Some(false),
    //     draw_border: None,
    //     color_format: ColorFormat::Rgba8,
    //     flags: state.clone(),
    // })
    // .unwrap();

    Ok(())
}

fn process_thread(src: &ShareThing<RgbaImage>) -> Result<(), Box<dyn Error>> {
    let identifier = CharacterIdentifier::load()?;

    loop {
        let image = src.take();
        image.save("0.png")?;

        process_image(&identifier, &image)?;

        thread::sleep(Duration::from_millis(500));
    }
}

fn debug_comparison(
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
            c.put_pixel(x, y, Rgba::from([255, 0, 0, 255]));
        } else if b.get_pixel(x, y).0[3] > 0 {
            c.put_pixel(x, y, Rgba::from([0, 0, 255, 255]));
        }
    }

    Ok(c)
}

fn process_image(
    identifier: &CharacterIdentifier,
    image: &RgbaImage,
) -> Result<(), Box<dyn Error>> {
    let preprocessed = preprocess_image(&image);
    preprocessed.save("preprocessed.png")?;

    let mut shapes = Shape::find_all(&preprocessed)
        .into_iter()
        .filter(|shape| is_digit_shape(shape))
        .collect::<Vec<_>>();

    shapes.sort_by(|a, b| {
        let a = a.bounds();
        let b = b.bounds();

        if a.y + a.height < b.y {
            Ordering::Less
        } else if b.y + b.height < a.y {
            Ordering::Greater
        } else if a.x + a.width < b.x {
            Ordering::Less
        } else if b.x + b.width < a.x {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    });

    // get_digit_example(&preprocessed, &shapes[0]).save("digits/9.png")?;
    // get_digit_example(&preprocessed, &shapes[1]).save("digits/8.png")?;
    // get_digit_example(&preprocessed, &shapes[2]).save("digits/7.png")?;
    // get_digit_example(&preprocessed, &shapes[3]).save("digits/6.png")?;
    // get_digit_example(&preprocessed, &shapes[4]).save("digits/5.png")?;
    // get_digit_example(&preprocessed, &shapes[5]).save("digits/4.png")?;
    // get_digit_example(&preprocessed, &shapes[6]).save("digits/3.png")?;
    // get_digit_example(&preprocessed, &shapes[7]).save("digits/2.png")?;
    // get_digit_example(&preprocessed, &shapes[8]).save("digits/1.png")?;
    // get_digit_example(&preprocessed, &shapes[9]).save("digits/0.png")?;
    // get_digit_example(&preprocessed, &shapes[10]).save("digits/slash.png")?;

    // debug_comparison(&preprocessed, &shapes[1], &image::open("digits/slash.png")?)?
    //     .save("compare1.png")?;

    // debug_comparison(&preprocessed, &shapes[12], &image::open("digits/slash.png")?)?
    //     .save("compare2.png")?;

    // debug_comparison(&preprocessed, &shapes[2], &image::open("digits/1.png")?)?
    //     .save("compare3.png")?;

    // debug_comparison(&preprocessed, &shapes[2], &image::open("digits/3.png")?)?
    //     .save("compare4.png")?;

    let mut debug = ImageBuffer::new(preprocessed.width(), preprocessed.height());

    for (i, shape) in shapes.into_iter().enumerate() {
        if is_digit_shape(&shape)
            && identifier.identify(&shape.create_image(&preprocessed)).1 >= 700000
        {
            shape
                .create_image(&preprocessed)
                .save(format!("shapes/{i}.png"))?;

            println!("{}", identifier.identify(&shape.create_image(image)).0);

            for &(x, y) in shape.all_pixels() {
                debug.put_pixel(x, y, preprocessed.get_pixel(x, y).clone());
            }
        }
    }

    debug.save("debug.png")?;

    let mut digits = identify_digits(&identifier, &preprocessed)?;
    digits.sort_by(|a, b| {
        let a = a.shape.bounds();
        let b = b.shape.bounds();

        if a.y + a.height < b.y {
            Ordering::Less
        } else if b.y + b.height < a.y {
            Ordering::Greater
        } else if a.x + a.width < b.x {
            Ordering::Less
        } else if b.x + b.width < a.x {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    });

    for digit in digits.iter() {
        println!("{}", digit.character);
    }

    let words = group_words(digits.into_iter())
        .into_iter()
        .map(|word| word.iter().map(|ch| ch.character).collect::<String>())
        .collect::<Vec<_>>();

    println!("{:?}", words);

    if words.len() >= 3 {
        let lives: usize = words[0].parse()?;
        let money: usize = words[1].parse()?;
        let round_info = words[words.len() - 1].split('/').collect::<Vec<_>>();

        let current_round: usize = round_info[0].parse()?;
        let total_rounds: Option<usize> = round_info.get(1).map(|s| s.parse()).transpose()?;

        println!(
            "lives: {}  money: {}  round: {}",
            lives, money, current_round
        );
    }

    Ok(())
}
