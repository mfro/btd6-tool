use image::{
    imageops::{crop_imm, resize, FilterType},
    math::Rect,
    DynamicImage, GenericImageView, ImageBuffer, ImageError, Pixel, Rgba, RgbaImage,
};
use ocrs::{OcrEngine, OcrEngineParams};
use rten::Model;
use rten_tensor::{prelude::*, NdTensor, NdTensorView};
use std::{
    cell::Cell,
    cmp::Ordering,
    error::Error,
    fs,
    sync::{Arc, Condvar, Mutex},
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

#[derive(Default)]
struct ShareThing<T> {
    data: Mutex<Option<T>>,
    wake: Condvar,
}

impl<T> ShareThing<T> {
    fn new() -> Self {
        let data = Mutex::default();
        let wake = Condvar::default();

        Self { data, wake }
    }

    fn put(&self, value: T) {
        let mut data = self.data.lock().unwrap();

        *data = Some(value);
        self.wake.notify_one();
    }

    fn take(&self) -> T {
        let mut data = self.data.lock().unwrap();
        let mut data = self.wake.wait_while(data, |data| data.is_none()).unwrap();

        data.take().unwrap()
    }
}

struct State {
    image: ShareThing<RgbaImage>,
}

struct X {
    state: Arc<State>,
}

impl WindowsCaptureHandler for X {
    type Flags = Arc<State>;
    type Error = Box<dyn Error + Send + Sync>;

    fn new(state: Self::Flags) -> Result<Self, Self::Error> {
        Ok(X { state })
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

        self.state.image.put(image);

        Ok(())
    }
}

struct CharacterFilter<'a> {
    image: &'a RgbaImage,
    cache: Vec<Vec<[bool; 4]>>,
}

const DIR4: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
const DIR8: [(i32, i32); 8] = [
    (-1, 0),
    (1, 0),
    (0, -1),
    (0, 1),
    (-1, -1),
    (1, -1),
    (-1, -1),
    (1, 1),
];
const PLUS8: [(i32, i32); 8] = [
    (-1, 0),
    (1, 0),
    (0, -1),
    (0, 1),
    (-2, 0),
    (2, 0),
    (0, -2),
    (0, 2),
];

struct IdentifiedCharacter {
    shape: Shape,
    character: char,
}

impl<'a> CharacterFilter<'a> {
    fn new(image: &'a RgbaImage) -> Self {
        let cache = vec![vec![[true; 4]; image.width() as usize]; image.height() as usize];

        Self { image, cache }
    }

    pub fn filter(&mut self, image: &mut RgbaImage) {
        let t0 = Instant::now();

        for (x, y, pixel) in self.image.enumerate_pixels() {
            if self.is_interior(pixel) && self.is_character_pixel(x, y) {
                image.put_pixel(x, y, *pixel);
            }
        }

        let t1 = Instant::now();
        println!("character filter {:?}", t1 - t0);
    }

    pub fn is_character_pixel(&mut self, x: u32, y: u32) -> bool {
        (0..DIR4.len()).all(|dir| self.find_outline(x as i32, y as i32, dir))
    }

    fn is_outline(&self, pixel: &Rgba<u8>) -> bool {
        let r = pixel.0[0] as usize;
        let g = pixel.0[1] as usize;
        let b = pixel.0[2] as usize;

        r + g + b == 0
    }

    fn is_interior(&self, pixel: &Rgba<u8>) -> bool {
        let r = pixel.0[0] as usize;
        let g = pixel.0[1] as usize;
        let b = pixel.0[2] as usize;

        r + g + b >= 352
    }

    fn is_out_of_bounds(&self, x: i32, y: i32) -> bool {
        x < 0 || x as u32 >= self.image.width() || y < 0 || y as u32 >= self.image.height()
    }

    fn find_outline(&mut self, x: i32, y: i32, dir: usize) -> bool {
        if self.is_out_of_bounds(x, y) || !self.cache[y as usize][x as usize][dir] {
            false
        } else {
            let (dx, dy) = DIR4[dir];

            let pixel = self.image.get_pixel(x as u32, y as u32);

            if pixel.0[3] != 0 && (self.is_outline(pixel) || self.find_outline(x + dx, y + dy, dir))
            {
                true
            } else {
                self.cache[y as usize][x as usize][dir] = false;
                false
            }
        }
    }
}

fn process_image2(image: &RgbaImage) -> Result<(), Box<dyn Error>> {
    let t0 = Instant::now();
    let mut step0 = ImageBuffer::new(image.width(), image.height());

    for (x, y, pixel0) in image.enumerate_pixels() {
        let r0 = pixel0.0[0] as usize;
        let g0 = pixel0.0[1] as usize;
        let b0 = pixel0.0[2] as usize;

        let avg0 = r0 + g0 + b0;

        if x > 1 && y > 1 && x + 2 < image.width() && y + 2 < image.height() {
            let values: Vec<_> = PLUS8
                .iter()
                .map(|&(dx, dy)| {
                    let pixel1 = image.get_pixel((x as i32 + dx) as u32, (y as i32 + dy) as u32);
                    let r1 = pixel1.0[0] as usize;
                    let g1 = pixel1.0[1] as usize;
                    let b1 = pixel1.0[2] as usize;

                    r1 + g1 + b1
                })
                .collect();

            let min = values.iter().min().unwrap().min(&avg0);
            let max = values.iter().max().unwrap().max(&avg0);

            // for dir in PLUS8 {
            //     let pixel1 = image.get_pixel((x as i32 + dir.0) as u32, (y as i32 + dir.1) as u32);
            //     let r1 = pixel1.0[0] as usize;
            //     let g1 = pixel1.0[1] as usize;
            //     let b1 = pixel1.0[2] as usize;

            //     let avg1 = r1 + g1 + b1;

            if min.abs_diff(*max) >= 512 + 128 + 64 + 32 + 16 {
                step0.put_pixel(x, y, pixel0.clone());
            }
            // }
        }
    }

    let t1 = Instant::now();
    println!("filter black & white {:?}", t1 - t0);

    step0.save("step2.0.png")?;

    Ok(())
}

fn filter_greyscale(image: &RgbaImage) -> RgbaImage {
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

        if r + g + b >= 760 {
            out.put_pixel(x, y, pixel.clone());
        }
    }

    let t1 = Instant::now();
    println!("filter greyscale {:?}", t1 - t0);

    out
}

fn is_character_shape(shape: &Shape) -> bool {
    shape.interior_pixels.len() >= 128
}

fn identify_digits(
    identifier: &DigitIdentifier,
    image: &RgbaImage,
) -> Result<Vec<IdentifiedCharacter>, Box<dyn Error>> {
    let shapes = find_shapes(image);

    let t0 = Instant::now();

    let shapes: Vec<_> = shapes
        .into_iter()
        .filter(|shape| is_character_shape(shape))
        .collect();

    let t1 = Instant::now();
    println!("filter shapes {:?}", t1 - t0);

    let t0 = Instant::now();

    let identified = shapes
        .iter()
        .map(|shape| {
            let shape_image = shape.create_image(image);
            let character = identifier.identify(&shape_image);

            IdentifiedCharacter {
                character,
                shape: shape.clone(),
            }
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

fn preprocess(image: &RgbaImage) -> RgbaImage {
    let cropped = crop_imm(image, 100, 40, 1220, 60).to_image();

    let greyscale = filter_greyscale(&cropped);

    greyscale
}

fn image_to_tensor(image: &DynamicImage) -> Result<NdTensor<f32, 3>, Box<dyn Error>> {
    let input_img = image.to_rgba8();
    let (width, height) = input_img.dimensions();
    let layout = input_img.sample_layout();

    let chw_tensor = NdTensorView::from_data_with_strides(
        [height as usize, width as usize, 3],
        input_img.as_raw().as_slice(),
        [
            layout.height_stride,
            layout.width_stride,
            layout.channel_stride,
        ],
    )?
    .permuted([2, 0, 1]) // HWC => CHW
    .to_tensor() // Make tensor contiguous, which makes `map` faster
    .map(|x| *x as f32 / 255.); // Rescale from [0, 255] to [0, 1]

    Ok(chw_tensor)
}

fn ocrs_testing(image: &DynamicImage) -> Result<(), Box<dyn Error>> {
    let t0 = Instant::now();

    // // Use the `download-models.sh` script to download the models.
    let detection_model_data = fs::read("text-detection.rten")?;
    let rec_model_data = fs::read("text-recognition.rten")?;

    let detection_model = Model::load(&detection_model_data)?;
    let recognition_model = Model::load(&rec_model_data)?;

    println!("load model {:?}", Instant::now() - t0);

    let engine = OcrEngine::new(OcrEngineParams {
        detection_model: Some(detection_model),
        recognition_model: Some(recognition_model),
        ..Default::default()
    })?;

    println!("create engine {:?}", Instant::now() - t0);

    // Read image using image-rs library and convert to a
    // (channels, height, width) tensor with f32 values in [0, 1].
    let tensor = image_to_tensor(image)?;

    // Apply standard image pre-processing expected by this library (convert
    // to greyscale, map range to [-0.5, 0.5]).
    let ocr_input = engine.prepare_input(tensor.view())?;

    println!("OCR 1 {:?}", Instant::now() - t0);

    // Phase 1: Detect text words
    let word_rects = engine.detect_words(&ocr_input)?;

    println!("OCR 2 {:?}", Instant::now() - t0);

    // Phase 2: Perform layout analysis
    let line_rects = engine.find_text_lines(&ocr_input, &word_rects);

    println!("OCR 3 {:?}", Instant::now() - t0);

    // Phase 3: Recognize text
    let line_texts = engine.recognize_text(&ocr_input, &line_rects)?;

    println!("OCR 4 {:?}", Instant::now() - t0);

    println!();

    for line in line_texts
        .iter()
        .flatten()
        // Filter likely spurious detections. With future model improvements
        // this should become unnecessary.
        .filter(|l| l.to_string().len() > 1)
    {
        println!("{}", line);
    }

    println!();

    Ok(())
}

#[derive(Default, Clone)]
struct Shape {
    edge_pixels: Vec<(u32, u32)>,
    interior_pixels: Vec<(u32, u32)>,
}

impl Shape {
    pub fn all_pixels(&self) -> impl Iterator<Item = &(u32, u32)> {
        self.edge_pixels.iter().chain(self.interior_pixels.iter())
    }

    pub fn bounds(&self) -> Rect {
        let min_x = self.edge_pixels.iter().map(|p| p.0).min().unwrap();
        let max_x = self.edge_pixels.iter().map(|p| p.0).max().unwrap();
        let min_y = self.edge_pixels.iter().map(|p| p.1).min().unwrap();
        let max_y = self.edge_pixels.iter().map(|p| p.1).max().unwrap();

        Rect {
            x: min_x,
            y: min_y,
            width: max_x - min_x + 1,
            height: max_y - min_y + 1,
        }
    }

    pub fn create_image(&self, image: &RgbaImage) -> RgbaImage {
        let bounds = self.bounds();
        let mut shape_image = ImageBuffer::new(bounds.width, bounds.height);

        for &(x, y) in self.all_pixels() {
            shape_image.put_pixel(x - bounds.x, y - bounds.y, image.get_pixel(x, y).clone());
        }

        shape_image
    }
}

struct DigitIdentifier {
    digits: Vec<DynamicImage>,
}

impl DigitIdentifier {
    pub fn load() -> Result<DigitIdentifier, Box<dyn Error>> {
        let digits: Result<Vec<_>, _> = (0..10)
            .map(|i| image::open(format!("digits/{i}.png")))
            .collect();

        let digits = digits?;

        Ok(Self { digits })
    }

    pub fn identify(&self, image: &RgbaImage) -> char {
        let resized = resize(image, 32, 32, FilterType::Nearest);

        let scores = self
            .digits
            .iter()
            .map(|digit| self.score(&resized, digit))
            .collect::<Vec<_>>();

        println!("{:?}", scores);

        let best = self
            .digits
            .iter()
            .map(|digit| self.score(&resized, digit))
            .enumerate()
            .max_by_key(|&(_, score)| score);

        b"0123456789"[best.unwrap().0].try_into().unwrap()
    }

    fn score(&self, image: &RgbaImage, digit: &DynamicImage) -> usize {
        image
            .enumerate_pixels()
            .filter(|&(x, y, &pixel)| pixel == digit.get_pixel(x, y))
            .count()
    }
}

struct ShapeFinder<'a, G: GenericImageView<Pixel = Rgba<u8>>> {
    shapes: Vec<Shape>,
    lookup: Vec<Vec<Option<usize>>>,
    image: &'a G,
}

impl<'a, G: GenericImageView<Pixel = Rgba<u8>>> ShapeFinder<'a, G> {}

fn find_shapes(image: &impl GenericImageView<Pixel = Rgba<u8>>) -> Vec<Shape> {
    let t0 = Instant::now();

    let mut shapes = Vec::new();
    let mut lookup = vec![vec![None; image.width() as usize]; image.height() as usize];

    for (x, y, pixel) in image.pixels() {
        if pixel.0[3] != 0 && lookup[y as usize][x as usize].is_none() {
            let index = shapes.len();
            let shape = explore_shape(&mut lookup, image, x, y, index);

            shapes.push(shape);
        }
    }

    let t1 = Instant::now();
    println!("find shapes {:?}", t1 - t0);

    shapes
}

fn explore_shape(
    lookup: &mut Vec<Vec<Option<usize>>>,
    image: &impl GenericImageView<Pixel = Rgba<u8>>,
    x: u32,
    y: u32,
    index: usize,
) -> Shape {
    let mut shape = Shape::default();

    let mut stack = vec![(x, y)];

    while let Some((x, y)) = stack.pop() {
        lookup[y as usize][x as usize] = Some(index);

        let mut adjacent = vec![];

        if y > 0 {
            adjacent.push((x, y - 1));
        }

        if x > 0 {
            adjacent.push((x - 1, y));
        }

        if x + 1 < image.width() {
            adjacent.push((x + 1, y));
        }

        if y + 1 < image.height() {
            adjacent.push((x, y + 1));
        }

        let mut is_edge = false;
        for (x, y) in adjacent {
            let rgba: Rgba<u8> = image.get_pixel(x, y).to_rgba();

            if rgba.0[3] == 0 {
                is_edge = true;
            } else if lookup[y as usize][x as usize].is_none() {
                stack.push((x, y));
            }
        }

        if is_edge {
            shape.edge_pixels.push((x, y));
        } else {
            shape.interior_pixels.push((x, y));
        }
    }

    shape
}

fn main() -> Result<(), Box<dyn Error>> {
    let state = Arc::new(State {
        image: Default::default(),
    });

    {
        let state = state.clone();

        thread::spawn(move || {
            process_thread(&state.image).unwrap();
        });
    }

    let x = Window::from_name("BloonsTD6-Epic").unwrap();

    X::start(Settings {
        item: x.try_into().unwrap(),
        capture_cursor: Some(false),
        draw_border: None,
        color_format: ColorFormat::Rgba8,
        flags: state.clone(),
    })
    .unwrap();

    Ok(())
}

fn process_thread(src: &ShareThing<RgbaImage>) -> Result<(), Box<dyn Error>> {
    let identifier = DigitIdentifier::load()?;

    loop {
        let image = src.take();
        image.save("0.png")?;

        let preprocessed = preprocess(&image);
        preprocessed.save("preprocessed.png")?;

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

        thread::sleep(Duration::from_millis(500));
    }
}
