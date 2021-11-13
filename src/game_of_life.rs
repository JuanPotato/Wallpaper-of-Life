use std::time::Instant;

pub struct BasicGoL {
    width: usize,
    height: usize,
    big_width: usize,
    big_height: usize,
    prev: Vec<u8>,
    next: Vec<u8>,
}

impl BasicGoL {
    pub fn new(width: usize, height: usize) -> Self {
        // Optimization #1 add a border of 0's around the whole grid so that you can always access
        // the neighbors of the elements within that grid instead of needing to check if we are
        // accessing elements out of the grid
        let big_width = width + 2;
        let big_height = height + 2;
        let n = big_width * big_height;

        Self {
            width,
            height,
            big_width,
            big_height,
            prev: vec![0u8; n],
            next: vec![0u8; n],
        }
    }

    fn get(&self, row: usize, col: usize) -> u8 {
        unsafe { *self.prev.get_unchecked(row * self.big_width + col) }
    }

    fn set(&mut self, row: usize, col: usize, val: u8) {
        unsafe {
            *self.next.get_unchecked_mut(row * self.big_width + col) = val;
        }
    }

    fn sum_neighbors(&self, row: usize, col: usize) -> u8 {
        0 + self.get(row - 1, col)
            + self.get(row + 1, col)
            + self.get(row, col - 1)
            + self.get(row, col + 1)
            + self.get(row - 1, col - 1)
            + self.get(row - 1, col + 1)
            + self.get(row + 1, col - 1)
            + self.get(row + 1, col + 1)
    }

    fn fill_with_gliders(&mut self) {
        for r in (1..(self.height + 1)).step_by(5) {
            for c in (1..(self.width + 1)).step_by(5) {
                self.set(r + 0, c + 1, 1);

                self.set(r + 1, c + 2, 1);

                self.set(r + 2, c + 0, 1);
                self.set(r + 2, c + 1, 1);
                self.set(r + 2, c + 2, 1);
            }
        }
    }

    pub fn glide(&mut self, iterations: usize) {
        self.fill_with_gliders();
        std::mem::swap(&mut self.prev, &mut self.next);

        let start = Instant::now();
        for _ in 0..iterations {
            for r in 1..(self.height + 1) {
                for c in 1..(self.width + 1) {
                    let sum = self.sum_neighbors(r, c);

                    let val = if self.get(r, c) == 1 {
                        sum == 2 || sum == 3
                    } else {
                        sum == 3
                    };

                    self.set(r, c, val as u8);
                }
            }

            std::mem::swap(&mut self.prev, &mut self.next);
        }

        let stop = Instant::now();
        let micro = stop.duration_since(start).as_micros() as f64;
        let milli = micro / 1000.0;

        println!(
            "Total: {:.6} ms ({:.6} ms / iter)",
            milli,
            milli / iterations as f64
        );
    }
}
