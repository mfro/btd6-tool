use image::{imageops::crop_imm, ImageBuffer, RgbaImage};
use std::{
    cell::Cell,
    error::Error,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
use windows_capture::{
    capture::WindowsCaptureHandler,
    frame::Frame,
    graphics_capture_api::InternalCaptureControl,
    settings::{ColorFormat, Settings},
    window::Window,
};

mod core;
mod debug;
use core::{
    identify::{self, CharacterIdentifier, IdentifiedCharacter},
    layout,
    shape::Shape,
    sync::ShareThing,
};

pub fn main() -> Result<(), Box<dyn Error>> {
    // debug::main()

    let state = Arc::new(CaptureState {
        frame: Default::default(),
    });

    {
        let state = state.clone();

        thread::spawn(move || {
            process_thread(&state.frame).unwrap();
        });
    }

    let x = Window::from_name("BloonsTD6-Epic").unwrap();

    Capture::start(Settings {
        item: x.try_into().unwrap(),
        capture_cursor: Some(false),
        draw_border: None,
        color_format: ColorFormat::Rgba8,
        flags: state.clone(),
    })
    .unwrap();

    state.frame.put(None);

    Ok(())
}

struct CaptureState {
    frame: ShareThing<Option<RgbaImage>>,
}

struct Capture {
    state: Arc<CaptureState>,
}

impl WindowsCaptureHandler for Capture {
    type Flags = Arc<CaptureState>;
    type Error = Box<dyn Error + Send + Sync>;

    fn new(state: Self::Flags) -> Result<Self, Self::Error> {
        Ok(Capture { state })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        _control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        let width = frame.width();
        let height = frame.height();

        let mut buffer = frame.buffer()?;
        let raw_buffer = buffer.as_raw_nopadding_buffer()?;

        let image: RgbaImage = ImageBuffer::from_raw(width, height, raw_buffer.to_vec()).unwrap();
        self.state.frame.put(Some(image));

        Ok(())
    }
}

fn preprocess_image(image: &RgbaImage) -> RgbaImage {
    let image = crop_imm(image, 100, 50, 1220, 40).to_image();

    let t0 = Instant::now();
    let mut out = ImageBuffer::new(image.width(), image.height());

    for (x, y, pixel) in image.enumerate_pixels() {
        let r = pixel.0[0] as usize;
        let g = pixel.0[1] as usize;
        let b = pixel.0[2] as usize;

        if r + g + b >= 256 {
            out.put_pixel(x, y, pixel.clone());
        }
    }

    let t1 = Instant::now();
    println!("filter greyscale {:?}", t1 - t0);

    out
}

fn identify_digits(
    identifier: &CharacterIdentifier,
    image: &RgbaImage,
) -> Result<Vec<IdentifiedCharacter>, Box<dyn Error>> {
    let t0 = Instant::now();

    let shapes: Vec<_> = Shape::find_all(image)
        .into_iter()
        .filter(|shape| identify::is_digit_shape(shape))
        .collect();

    let t1 = Instant::now();
    println!("find shapes {:?}", t1 - t0);

    let t0 = Instant::now();

    let identified = shapes
        .iter()
        .filter_map(|shape| {
            let shape_image = shape.create_image(image);
            let character = identifier.identify(&shape_image)?;

            Some(IdentifiedCharacter::new(shape.clone(), character))
        })
        .collect();

    let t1 = Instant::now();
    println!("identify shapes {:?}", t1 - t0);

    Ok(identified)
}

fn process_thread(src: &ShareThing<Option<RgbaImage>>) -> Result<(), Box<dyn Error>> {
    let identifier = CharacterIdentifier::load()?;

    while let Some(image) = src.take() {
        image.save("0.png")?;

        process_image(&identifier, &image)?;

        thread::sleep(Duration::from_millis(500));
    }

    Ok(())
}

fn process_image(
    identifier: &CharacterIdentifier,
    image: &RgbaImage,
) -> Result<(), Box<dyn Error>> {
    let preprocessed = preprocess_image(&image);

    let mut digits = identify_digits(&identifier, &preprocessed)?;
    digits.sort_by_key(|a| a.shape.bounds().x);

    println!("{}", digits.iter().map(|d| d.character).collect::<String>());

    let words = layout::group_words(digits.into_iter())
        .into_iter()
        .map(|word| word.iter().map(|ch| ch.character).collect::<String>())
        .collect::<Vec<_>>();

    println!("{:?}", words);

    if words.len() != 3 {
        let case_number = CASE_NUMBER.get();
        CASE_NUMBER.set(case_number + 1);

        image.save(format!("cases/{case_number}.png"))?;
    }

    if words.len() >= 3 {
        let lives: usize = words[0].parse()?;
        let money: usize = words[1].parse()?;
        let round_info = words[words.len() - 1].split('/').collect::<Vec<_>>();

        let current_round: usize = round_info[0].parse()?;
        let _total_rounds: Option<usize> = round_info.get(1).map(|s| s.parse()).transpose()?;

        println!(
            "lives: {}  money: {}  round: {}",
            lives, money, current_round
        );
    }

    Ok(())
}

thread_local! {
    static CASE_NUMBER: Cell<usize> = Cell::default();
}
