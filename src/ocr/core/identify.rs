use std::error::Error;

use image::{
    imageops::{resize, FilterType},
    DynamicImage, GenericImageView, Rgba, RgbaImage,
};

use super::shape::Shape;

pub fn is_digit_shape(shape: &Shape) -> bool {
    (128..2048).contains(&shape.len())
}

#[derive(Default, Clone, Debug, PartialEq, Eq, Hash)]
pub struct IdentifiedCharacter {
    pub shape: Shape,
    pub character: char,
}

impl IdentifiedCharacter {
    pub fn new(shape: Shape, character: char) -> Self {
        Self { shape, character }
    }
}

pub struct CharacterIdentifier {
    examples: Vec<(char, DynamicImage)>,
}

impl CharacterIdentifier {
    pub fn load() -> Result<CharacterIdentifier, Box<dyn Error>> {
        let examples = vec![
            ('0', image::open(format!("example/0.png"))?),
            ('1', image::open(format!("example/1.png"))?),
            ('2', image::open(format!("example/2.png"))?),
            ('3', image::open(format!("example/3.png"))?),
            ('4', image::open(format!("example/4.png"))?),
            ('5', image::open(format!("example/5.png"))?),
            ('6', image::open(format!("example/6.png"))?),
            ('7', image::open(format!("example/7.png"))?),
            ('8', image::open(format!("example/8.png"))?),
            ('9', image::open(format!("example/9.png"))?),
            ('/', image::open(format!("example/slash.png"))?),
        ];

        Ok(Self { examples })
    }

    pub fn debug(&self, image: &RgbaImage) -> &DynamicImage {
        let mut scores = self
            .examples
            .iter()
            .map(|digit| {
                (
                    digit.0,
                    &digit.1,
                    self.score(&image, &digit.1),
                    self.pixel_score(&image, &digit.1),
                    self.aspect_ratio_score(&image, &digit.1),
                )
            })
            .collect::<Vec<_>>();

        scores.sort_by(|a, b| f64::total_cmp(&a.2, &b.2));

        for entry in scores.iter() {
            println!(
                "  {:.2?} {:.2?} {:.2?} {:.2?}",
                entry.0, entry.2, entry.3, entry.4
            );
        }

        scores.last().unwrap().1
    }

    pub fn identify(&self, image: &RgbaImage) -> Option<char> {
        let best = self
            .examples
            .iter()
            .map(|digit| (digit.0, self.score(&image, &digit.1)))
            .max_by(|a, b| f64::total_cmp(&a.1, &b.1))
            .unwrap();

        (best.1 >= 0.6).then(|| best.0)
    }

    fn score(&self, image: &RgbaImage, digit: &DynamicImage) -> f64 {
        self.pixel_score(image, digit) * self.aspect_ratio_score(image, digit)
    }

    fn pixel_score(&self, image: &RgbaImage, digit: &DynamicImage) -> f64 {
        let image = resize(image, digit.width(), digit.height(), FilterType::Nearest);

        let count = image
            .enumerate_pixels()
            .filter(|&(x, y, pixel)| pixel.0[3] > 0 || digit.get_pixel(x, y).0[3] > 0)
            .count() as f64;

        let positive: f64 = image
            .enumerate_pixels()
            .map(|(x, y, &pixel)| diff(&pixel, &digit.get_pixel(x, y)))
            .sum();

        1.0 - positive / count
    }

    fn aspect_ratio_score(&self, image: &RgbaImage, digit: &DynamicImage) -> f64 {
        let aspect_ratio =
            (image.width() * digit.height()) as f64 / (digit.width() * image.height()) as f64;

        let aspect_ratio_score = ((aspect_ratio - 1.0).abs() + 1.0).powi(-2);

        // println!(
        //     "{} {} {} {} {} {}",
        //     image.width(),
        //     image.height(),
        //     digit.width(),
        //     digit.height(),
        //     aspect_ratio,
        //     aspect_ratio_score
        // );

        aspect_ratio_score
    }
}

fn diff(a: &Rgba<u8>, b: &Rgba<u8>) -> f64 {
    u8::abs_diff(a.0[3], b.0[3]) as f64 / 256.0
    // let square = (u8::abs_diff(a.0[0], b.0[0]) as f64).powi(2)
    //     + (u8::abs_diff(a.0[1], b.0[1]) as f64).powi(2)
    //     + (u8::abs_diff(a.0[2], b.0[2]) as f64).powi(2)
    //     + (u8::abs_diff(a.0[3], b.0[3]) as f64).powi(2);

    // square.sqrt() / 512.0
}
