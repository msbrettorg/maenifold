use maenifold_core::vector;
fn main() {
    println!("load={:?}", vector::load_model());
    println!(
        "dims={:?} file={:?} max_tokens={:?}",
        vector::embedding_dimensions(),
        vector::model_filename(),
        vector::max_tokens()
    );
}
