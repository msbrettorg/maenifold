use std::path::PathBuf;

fn main() {
    // Test how PathBuf handles double slashes
    let path1 = PathBuf::from("memory-path-tests//double-slash");
    let path2 = PathBuf::from("memory-path-tests/double-slash");
    
    println!("Path 1 (with //): {:?}", path1);
    println!("Path 1 string: {}", path1.display());
    println!("Path 2 (with /):  {:?}", path2);
    println!("Path 2 string: {}", path2.display());
    println!("Are they equal? {}", path1 == path2);
    
    // Also test the split behavior
    println!("\nSplitting path1 by '/':");
    for segment in path1.to_string_lossy().split('/') {
        println!("  Segment: '{}'", segment);
    }
}
