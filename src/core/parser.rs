//! Gesture parser module
//!
//! This module handles parsing mouse movements into gesture directions.

use crate::core::gesture::{GestureDir, Point};
use std::f32::consts::PI;

/// Calculate 4-direction gesture from a vector
pub fn calculate_4direction(vector: &Point) -> GestureDir {
    let dx = vector.x;
    let dy = -vector.y; // Invert Y because screen coordinates have Y pointing down

    // Determine quadrant and primary axis
    if dx >= 0 && dy >= 0 {
        // First quadrant (up-right)
        if dx > dy {
            GestureDir::Right
        } else {
            GestureDir::Up
        }
    } else if dx <= 0 && dy >= 0 {
        // Second quadrant (up-left)
        if -dx > dy {
            GestureDir::Left
        } else {
            GestureDir::Up
        }
    } else if dx <= 0 && dy <= 0 {
        // Third quadrant (down-left)
        if -dx > -dy {
            GestureDir::Left
        } else {
            GestureDir::Down
        }
    } else {
        // Fourth quadrant (down-right)
        if dx > -dy {
            GestureDir::Right
        } else {
            GestureDir::Down
        }
    }
}

/// Calculate 8-direction gesture from a vector
pub fn calculate_8direction(vector: &Point) -> GestureDir {
    let dx = vector.x as f32;
    let dy = -vector.y as f32; // Invert Y

    // Calculate angle in degrees
    let angle = dy.atan2(dx).to_degrees();
    let angle = if angle < 0.0 { angle + 360.0 } else { angle };

    // Divide into 8 sectors of 45 degrees each
    let slash_range = 50.0; // Degrees of fuzzy matching for diagonals
    let sector_size = 45.0;

    let n = (angle / sector_size) as i32;
    let n_is_even = (n & 1) == 0;

    // Apply fuzzy matching for diagonals
    let mod_val = angle % sector_size;

    let final_n = if n_is_even && mod_val > (sector_size - slash_range / 2.0)
        || !n_is_even && mod_val > (sector_size + slash_range / 2.0)
    {
        (n + 1) % 8
    } else {
        n % 8
    };

    match final_n {
        0 => GestureDir::Right,
        1 => GestureDir::UpRight,
        2 => GestureDir::Up,
        3 => GestureDir::UpLeft,
        4 => GestureDir::Left,
        5 => GestureDir::DownLeft,
        6 => GestureDir::Down,
        7 => GestureDir::DownRight,
        _ => unreachable!(),
    }
}

/// Calculate the angle between two vectors in degrees
fn get_angle(vector_a: &Point, vector_b: &Point) -> f32 {
    let ax = vector_a.x as f32;
    let ay = vector_a.y as f32;
    let bx = vector_b.x as f32;
    let by = vector_b.y as f32;

    let product = ax * bx + ay * by;
    let mag_a = (ax * ax + ay * ay).sqrt();
    let mag_b = (bx * bx + by * by).sqrt();

    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.0;
    }

    let cos_value = (product / (mag_a * mag_b)).clamp(-1.0, 1.0);
    cos_value.acos().to_degrees()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_4direction_right() {
        let vector = Point::new(10, 5); // Mostly right
        assert_eq!(calculate_4direction(&vector), GestureDir::Right);
    }

    #[test]
    fn test_4direction_up() {
        let vector = Point::new(3, -10); // Mostly up (negative Y in screen coords)
        assert_eq!(calculate_4direction(&vector), GestureDir::Up);
    }

    #[test]
    fn test_8direction_diagonal() {
        let vector = Point::new(10, 10); // Diagonal up-right
        let dir = calculate_8direction(&vector);
        assert!(dir.is_diagonal());
    }

    #[test]
    fn test_get_angle() {
        let v1 = Point::new(1, 0);
        let v2 = Point::new(0, 1);
        let angle = get_angle(&v1, &v2);
        assert!((angle - 90.0).abs() < 0.1);
    }
}
