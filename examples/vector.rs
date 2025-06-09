fn main() {
    let v_raw = [1, 2, 3, 4, 5];
    let vec = Vec::from(&v_raw[..]);

    println!("{}", v_raw[0]);
    println!("{}", vec[0]);

    let my_vec = MyVec {
        raw: [1, 2, 3, 4, 5],
        size: 5,
    };

    println!("{}", my_vec[0]);
}

struct MyVec {
    raw: [i32; 5],
    size: usize,
}

impl std::ops::Index<usize> for MyVec {
    type Output = i32;

    fn index(&self, index: usize) -> &Self::Output {
        &self.raw[index]
    }
}
