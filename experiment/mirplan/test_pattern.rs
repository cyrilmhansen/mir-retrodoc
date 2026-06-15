fn main() {
    let start = 0u64;
    let limit = 3u64;
    let step = 1u64;
    let trip_count = if start < limit { (limit - start + step - 1) / step } else { 0 };
    println!("trip_count={}", trip_count);
}
