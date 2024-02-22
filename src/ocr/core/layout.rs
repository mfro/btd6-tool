use std::cmp::Ordering;

use image::math::Rect;

use super::identify::IdentifiedCharacter;

pub fn is_line_break(a: &IdentifiedCharacter, b: &IdentifiedCharacter) -> bool {
    let a = a.shape.bounds();
    let b = b.shape.bounds();

    (b.y).abs_diff(a.y) >= b.height
}

pub fn is_word_break(a: &IdentifiedCharacter, b: &IdentifiedCharacter) -> bool {
    let a = a.shape.bounds();
    let b = b.shape.bounds();

    (b.x).abs_diff(a.x) >= b.width * 3
}

pub fn group_words(
    src: impl Iterator<Item = IdentifiedCharacter>,
) -> Vec<Vec<IdentifiedCharacter>> {
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

pub fn compare_bounds(a: &Rect, b: &Rect) -> Ordering {
    if a.y + a.height < b.y {
        Ordering::Less
    } else if b.y + b.height < a.y {
        Ordering::Greater
    } else if a.x + a.width < b.x {
        Ordering::Less
    } else if b.x + b.width < a.x {
        Ordering::Greater
    } else if a.x.abs_diff(b.x) > a.y.abs_diff(b.y) {
        a.x.cmp(&b.x)
    } else {
        a.y.cmp(&b.y)
    }
}
