use afl::fuzz;
use secure_safe::file_format::header::{Header, Red};

fn main() {
    fuzz!(|data: &[u8]| {
        let _ = Header::<Red>::fuzzing_load_inner(data);
    });
}
