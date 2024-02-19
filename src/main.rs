use image::{DynamicImage, GenericImageView, ImageBuffer, Pixel, Rgba, RgbaImage};
use ocrs::{OcrEngine, OcrEngineParams};
use rten::Model;
use rten_tensor::{prelude::*, NdTensor, NdTensorView};
use std::{error::Error, fs, time::Instant};
use windows_capture::{
    capture::WindowsCaptureHandler,
    frame::Frame,
    graphics_capture_api::InternalCaptureControl,
    settings::{ColorFormat, Settings},
    window::Window,
};

// fn read_some_memory(pid: Pid, address: usize, size: usize) -> io::Result<()> {
//     let handle: ProcessHandle = pid.try_into()?;
//     let _bytes = copy_address(address, size, &handle)?;
//     println!("Read {} bytes", size);
//     Ok(())
// }

struct X;

impl WindowsCaptureHandler for X {
    type Flags = String;
    type Error = Box<dyn Error + Send + Sync>;

    fn new(flags: Self::Flags) -> Result<Self, Self::Error> {
        println!("{}", flags);

        Ok(X {})
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

        image.save("0.png")?;

        // process_image1(&image)?;
        // process_image2(&image)?;
        let processed_image = process_image3(&image)?;

        ocrs_testing(&processed_image).unwrap();

        control.stop();

        Ok(())
    }
}

fn process_image1(
    image: &impl GenericImageView<Pixel = Rgba<u8>>,
) -> Result<DynamicImage, Box<dyn Error + Send + Sync>> {
    let t0 = Instant::now();
    let mut out = ImageBuffer::new(image.width(), image.height());

    for (x, y, color) in image.pixels() {
        let rgba = color.to_rgba();
        let r = rgba.0[0] as usize;
        let g = rgba.0[1] as usize;
        let b = rgba.0[2] as usize;

        let err = (2 * r).abs_diff(b + g) + (2 * b).abs_diff(r + g) + (2 * g).abs_diff(r + b);

        if err <= 4 {
            out.put_pixel(x, y, image.get_pixel(x, y));
        }
    }

    let t1 = Instant::now();
    println!("{:?}", t1 - t0);

    out.save("1.png")?;

    Ok(out.into())
}

fn process_image2(
    image: &impl GenericImageView<Pixel = Rgba<u8>>,
) -> Result<DynamicImage, Box<dyn Error + Send + Sync>> {
    let t0 = Instant::now();
    let mut out = ImageBuffer::new(image.width(), image.height());

    let mut state = State::None;

    enum State {
        None,
        Sealed(u32, u32),
    }

    for (x, y, color) in image.pixels() {
        let rgba = color.to_rgba();
        let r = rgba.0[0] as usize;
        let g = rgba.0[1] as usize;
        let b = rgba.0[2] as usize;

        let sum = r + b + g;
        let err = (2 * r).abs_diff(b + g) + (2 * b).abs_diff(r + g) + (2 * g).abs_diff(r + b);

        match state {
            State::None => {
                if err >= 12 {
                    state = State::None;
                } else if sum <= 24 {
                    state = State::Sealed(x, y);
                }
            }
            State::Sealed(x0, y0) => {
                if err >= 12 || y != y0 {
                    for x in x0..x {
                        out.put_pixel(x, y0, image.get_pixel(x, y0));
                    }

                    state = State::None;
                }
            }
        }
    }

    let t1 = Instant::now();
    println!("{:?}", t1 - t0);

    out.save("2.png")?;

    Ok(out.into())
}

#[derive(Default)]
struct Shape {
    edge: Vec<usize>,
    interior: Vec<usize>,
}

impl Shape {
    fn is_character(&self) -> bool {
        let edge_score = self.edge_score();
        let interior_score = self.interior_score();

        println!(
            "{} {} {} {}",
            self.edge.len(),
            self.interior.len(),
            edge_score,
            interior_score
        );

        edge_score < 64
    }

    fn edge_score(&self) -> usize {
        self.edge.iter().sum::<usize>() / self.edge.len()
    }

    fn interior_score(&self) -> usize {
        self.interior.iter().max().cloned().unwrap_or_default()
    }
}

struct ShapeFinder<'a, G: GenericImageView<Pixel = Rgba<u8>>> {
    shapes: Vec<Shape>,
    lookup: Vec<Vec<Option<usize>>>,
    image: &'a G,
}

impl<'a, G: GenericImageView<Pixel = Rgba<u8>>> ShapeFinder<'a, G> {}

fn explore2(
    shapes: &mut Vec<Shape>,
    lookup: &mut Vec<Vec<Option<usize>>>,
    image: &impl GenericImageView<Pixel = Rgba<u8>>,
    x: u32,
    y: u32,
    index: usize,
) {
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

        let rgba: Rgba<u8> = image.get_pixel(x, y).to_rgba();
        let r = rgba.0[0] as usize;
        let g = rgba.0[1] as usize;
        let b = rgba.0[2] as usize;

        let sum = r + g + b;
        if is_edge {
            shapes[index].edge.push(sum);
        } else {
            shapes[index].interior.push(sum);
        }
    }
}

struct CharacterFinder<'a> {
    image: &'a RgbaImage,
    cache: Vec<Vec<[bool; 4]>>,
}

const DIRECTIONS: [(i32, i32); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

impl<'a> CharacterFinder<'a> {
    fn new(image: &'a RgbaImage) -> Self {
        let cache = vec![vec![[true; 4]; image.width() as usize]; image.height() as usize];

        Self { image, cache }
    }

    pub fn fill(&mut self, image: &mut RgbaImage) {
        let t0 = Instant::now();

        for (x, y, pixel) in self.image.enumerate_pixels() {
            if self.is_interior(pixel) && self.is_character_pixel(x, y) {
                image.put_pixel(x, y, *pixel);
            }
        }

        let t1 = Instant::now();
        println!("filter character finder {:?}", t1 - t0);
    }

    pub fn is_character_pixel(&mut self, x: u32, y: u32) -> bool {
        (0..4).any(|dir| self.find_outline(x as i32, y as i32, dir))
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
            let (dx, dy) = DIRECTIONS[dir];

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

fn process_image3(image: &RgbaImage) -> Result<DynamicImage, Box<dyn Error + Send + Sync>> {
    let t0 = Instant::now();
    let mut step0 = ImageBuffer::new(image.width(), image.height());

    for (x, y, pixel) in image.enumerate_pixels() {
        let r = pixel.0[0] as usize;
        let g = pixel.0[1] as usize;
        let b = pixel.0[2] as usize;

        let err = (2 * r).abs_diff(b + g) + (2 * b).abs_diff(r + g) + (2 * g).abs_diff(r + b);

        if err <= 4 {
            step0.put_pixel(x, y, pixel.clone());
        }
    }

    let t1 = Instant::now();
    println!("filter black & white {:?}", t1 - t0);

    step0.save("step0.png")?;

    let mut finder = CharacterFinder::new(&step0);

    let mut step1 = ImageBuffer::new(image.width(), image.height());
    finder.fill(&mut step1);

    // let mut shapes: Vec<Shape> = Vec::new();
    // let mut shape_lookup: Vec<Vec<Option<usize>>> =
    //     vec![vec![None; step0.width() as usize]; step0.height() as usize];

    // for (x, y, pixel) in image.pixels() {
    // if step0.get_pixel(x, y).0[3] != 0 && shape_lookup[y as usize][x as usize].is_none() {
    //     let index = shapes.len();
    //     shapes.push(Shape::default());

    //     explore2(&mut shapes, &mut shape_lookup, &step0, x, y, index);
    // }
    // }

    // let scores: Vec<_> = shapes.iter().map(|s| s.is_character()).collect();

    // let mut step1 = ImageBuffer::new(image.width(), image.height());

    // for (x, y, pixel) in image.pixels() {
    //     if let Some(i) = shape_lookup[y as usize][x as usize] {
    //         if scores[i] {
    //             step1.put_pixel(x, y, pixel);
    //         }
    //     }
    // }

    step1.save("step1.png")?;

    Ok(step1.into())
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
    // // Use the `download-models.sh` script to download the models.
    let detection_model_data = fs::read("text-detection.rten")?;
    let rec_model_data = fs::read("text-recognition.rten")?;

    let detection_model = Model::load(&detection_model_data)?;
    let recognition_model = Model::load(&rec_model_data)?;

    let engine = OcrEngine::new(OcrEngineParams {
        detection_model: Some(detection_model),
        recognition_model: Some(recognition_model),
        ..Default::default()
    })?;

    // Read image using image-rs library and convert to a
    // (channels, height, width) tensor with f32 values in [0, 1].
    let tensor = image_to_tensor(image)?;

    // Apply standard image pre-processing expected by this library (convert
    // to greyscale, map range to [-0.5, 0.5]).
    let ocr_input = engine.prepare_input(tensor.view())?;

    // Phase 1: Detect text words
    let word_rects = engine.detect_words(&ocr_input)?;

    // Phase 2: Perform layout analysis
    let line_rects = engine.find_text_lines(&ocr_input, &word_rects);

    // Phase 3: Recognize text
    let line_texts = engine.recognize_text(&ocr_input, &line_rects)?;

    for line in line_texts
        .iter()
        .flatten()
        // Filter likely spurious detections. With future model improvements
        // this should become unnecessary.
        .filter(|l| l.to_string().len() > 1)
    {
        println!("{}", line);
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let x = Window::from_name("BloonsTD6-Epic").unwrap();

    X::start(Settings {
        item: x.try_into().unwrap(),
        capture_cursor: Some(false),
        draw_border: None,
        color_format: ColorFormat::Rgba8,
        flags: "".to_owned(),
    })
    .unwrap();

    Ok(())
}
