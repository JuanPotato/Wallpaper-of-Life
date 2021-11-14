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

    pub fn iter(&self) -> impl Iterator<Item = impl Iterator<Item = u8> + '_> + '_ {
        (1..(self.height + 1)).map(move |r| {
            let start = r * self.big_width + 1;
            self.prev[start..start + self.width].iter().copied()
        })
    }

    fn get_internal(&self, row: usize, col: usize) -> u8 {
        unsafe { *self.prev.get_unchecked(row * self.big_width + col) }
    }

    fn set_internal(&mut self, row: usize, col: usize, val: u8) {
        unsafe {
            *self.next.get_unchecked_mut(row * self.big_width + col) = val;
        }
    }

    fn sum_neighbors(&self, row: usize, col: usize) -> u8 {
        0 + self.get_internal(row - 1, col)
            + self.get_internal(row + 1, col)
            + self.get_internal(row, col - 1)
            + self.get_internal(row, col + 1)
            + self.get_internal(row - 1, col - 1)
            + self.get_internal(row - 1, col + 1)
            + self.get_internal(row + 1, col - 1)
            + self.get_internal(row + 1, col + 1)
    }

    fn swap_buffers(&mut self) {
        std::mem::swap(&mut self.prev, &mut self.next);
    }

    pub fn fill_with_gliders(&mut self) {
        for r in (1..(self.height + 1)).step_by(5) {
            for c in (1..(self.width + 1)).step_by(5) {
                self.set_internal(r + 0, c + 1, 1);

                self.set_internal(r + 1, c + 2, 1);

                self.set_internal(r + 2, c + 0, 1);
                self.set_internal(r + 2, c + 1, 1);
                self.set_internal(r + 2, c + 2, 1);
            }
        }

        self.swap_buffers();
    }

    pub fn tick(&mut self) {
        for r in 1..(self.height + 1) {
            for c in 1..(self.width + 1) {
                let sum = self.sum_neighbors(r, c);

                let val = if self.get_internal(r, c) == 1 {
                    sum == 2 || sum == 3
                } else {
                    sum == 3
                };

                self.set_internal(r, c, val as u8);
            }
        }

        self.swap_buffers();
    }

    pub fn bench(&mut self, iterations: usize) {
        self.fill_with_gliders();

        let start = Instant::now();

        for _ in 0..iterations {
            self.tick();
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

    pub fn print(&self, dead: &str, alive: &str) {
        let mut output = String::with_capacity((dead.len() * self.width + 1) * self.height);
        for row in self.iter() {
            for cell in row {
                if cell == 0 {
                    output.push_str(dead);
                } else {
                    output.push_str(alive);
                }
            }
            output.push('\n');
        }

        print!("{}", output);
    }
}
