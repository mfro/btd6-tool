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

    pub fn identify(&self, image: &RgbaImage) -> (char, usize) {
        let resized = resize(image, 32, 32, FilterType::Nearest);

        let scores = self
            .characters
            .iter()
            .map(|digit| (digit.0, self.score(&resized, &digit.1)))
            .collect::<Vec<_>>();

        println!("{:?}", scores);

        let best = self
            .characters
            .iter()
            .map(|digit| (digit.0, self.score(&resized, &digit.1)))
            .max_by_key(|&(_, score)| score)
            .unwrap();

        println!("{:?}", best);

        best
    }

    fn score(&self, image: &RgbaImage, digit: &DynamicImage) -> usize {
        // TODO: consider aspect ratio

        let image = resize(image, digit.width(), digit.height(), FilterType::Nearest);

        let count = image
            .enumerate_pixels()
            .filter(|&(x, y, pixel)| pixel.0[3] > 0 || digit.get_pixel(x, y).0[3] > 0)
            .count();

        let positive = image
            .enumerate_pixels()
            .filter(|&(x, y, &pixel)| (pixel.0[3] > 0) && (digit.get_pixel(x, y).0[3] > 0))
            .count();

        positive * 1000000 / count
    }
}
