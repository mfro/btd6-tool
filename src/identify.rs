use std::{collections::HashSet, error::Error};

use image::{
    imageops::{resize, FilterType},
    DynamicImage, GenericImageView, RgbaImage,
};

use crate::Shape;

const DIR8: [(i32, i32); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
];

pub fn contour(a: &Shape) -> Vec<(i32, i32)> {
    let edges: HashSet<(u32, u32)> = a.edge_pixels().iter().cloned().collect();

    let origin = a.edge_pixels()[0];

    let mut path = vec![];
    let mut node = origin;
    let mut direction = 0;

    println!("{:#?}", a.edge_pixels());

    loop {
        println!("{:?}", node);

        for i in (0..8).map(|i| direction + i) {
            let dir = DIR8[i % 8];

            let next = (
                (node.0 as i32 + dir.0) as u32,
                (node.1 as i32 + dir.1) as u32,
            );

            println!("  {:?} {:?}", dir, next);

            if next == origin {
                return path;
            }

            if edges.contains(&next) {
                path.push(dir);
                node = next;
                direction = (i + 5) % 8;
                break;
            }
        }
    }
}

pub struct CharacterIdentifier {
    characters: Vec<(char, DynamicImage)>,
}

impl CharacterIdentifier {
    pub fn load() -> Result<CharacterIdentifier, Box<dyn Error>> {
        let digits = vec![
            ('0', image::open(format!("digits/0.png"))?),
            ('1', image::open(format!("digits/1.png"))?),
            ('2', image::open(format!("digits/2.png"))?),
            ('3', image::open(format!("digits/3.png"))?),
            ('4', image::open(format!("digits/4.png"))?),
            ('5', image::open(format!("digits/5.png"))?),
            ('6', image::open(format!("digits/6.png"))?),
            ('7', image::open(format!("digits/7.png"))?),
            ('8', image::open(format!("digits/8.png"))?),
            ('9', image::open(format!("digits/9.png"))?),
            ('/', image::open(format!("digits/slash.png"))?),
        ];

        Ok(Self { characters: digits })
    }

    pub fn identify(&self, image: &RgbaImage) -> Option<char> {
        // let resized = resize(image, 32, 32, FilterType::Nearest);

        // let scores = self
        //     .characters
        //     .iter()
        //     .map(|digit| {
        //         (
        //             digit.0,
        //             self.score(&image, &digit.1),
        //             self.pixel_score(&image, &digit.1),
        //             self.aspect_ratio_score(&image, &digit.1),
        //         )
        //     })
        //     .collect::<Vec<_>>();

        // println!("{:.2?}", scores);

        let best = self
            .characters
            .iter()
            .map(|digit| (digit.0, self.score(&image, &digit.1)))
            .max_by(|a, b| f64::total_cmp(&a.1, &b.1))
            .unwrap();

        // let entry = self.characters.iter().find(|d| d.0 == best.0).unwrap();
        // println!("{}: {:.2} * {:.2} = {:.2}", best.0, self.pixel_score(image, &entry.1), self.aspect_ratio_score(image, &entry.1), self.score(image, &entry.1));

        (best.1 >= 0.5).then(|| best.0)
    }

    fn score(&self, image: &RgbaImage, digit: &DynamicImage) -> f64 {
        self.pixel_score(image, digit) * self.aspect_ratio_score(image, digit)
    }

    fn pixel_score(&self, image: &RgbaImage, digit: &DynamicImage) -> f64 {
        let image = resize(image, digit.width(), digit.height(), FilterType::Nearest);

        let count = image
            .enumerate_pixels()
            .filter(|&(x, y, pixel)| pixel.0[3] > 0 || digit.get_pixel(x, y).0[3] > 0)
            .count();

        let positive = image
            .enumerate_pixels()
            .filter(|&(x, y, &pixel)| (pixel.0[3] > 0) && (digit.get_pixel(x, y).0[3] > 0))
            .count();

        positive as f64 / count as f64
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
