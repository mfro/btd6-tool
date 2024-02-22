use image::{math::Rect, GenericImageView, ImageBuffer, Pixel, Rgba, RgbaImage};

#[derive(Default, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Shape {
    edge_pixels: Vec<(u32, u32)>,
    interior_pixels: Vec<(u32, u32)>,
}

impl Shape {
    pub fn all_pixels(&self) -> impl Iterator<Item = &(u32, u32)> {
        self.edge_pixels.iter().chain(self.interior_pixels.iter())
    }

    pub fn len(&self) -> usize {
      self.edge_pixels.len() + self.interior_pixels.len()
    }

    pub fn edge_pixels(&self) -> &[(u32, u32)] {
      self.edge_pixels.as_ref()
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

    pub fn find_all(image: &impl GenericImageView<Pixel = Rgba<u8>>) -> Vec<Shape> {
        let mut shapes = Vec::new();
        let mut lookup = vec![vec![None; image.width() as usize]; image.height() as usize];

        for (x, y, pixel) in image.pixels() {
            if pixel.0[3] != 0 && lookup[y as usize][x as usize].is_none() {
                let index = shapes.len();
                let shape = Shape::build(image, &mut lookup, x, y, index);

                shapes.push(shape);
            }
        }

        shapes
    }

    fn build(
        image: &impl GenericImageView<Pixel = Rgba<u8>>,
        lookup: &mut Vec<Vec<Option<usize>>>,
        x: u32,
        y: u32,
        index: usize,
    ) -> Shape {
        let mut shape = Shape::default();

        let mut stack = vec![(x, y)];

        while let Some((x, y)) = stack.pop() {
            lookup[y as usize][x as usize] = Some(index);

            let mut adjacent = vec![];
            let mut is_edge = false;

            if y > 0 {
                adjacent.push((x, y - 1));
            } else {
                is_edge = true;
            }

            if x > 0 {
                adjacent.push((x - 1, y));
            } else {
                is_edge = true;
            }

            if x + 1 < image.width() {
                adjacent.push((x + 1, y));
            } else {
                is_edge = true;
            }

            if y + 1 < image.height() {
                adjacent.push((x, y + 1));
            } else {
                is_edge = true;
            }

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
}
