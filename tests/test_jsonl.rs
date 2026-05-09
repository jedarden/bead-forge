use bead_forge::Issue;

fn main() {
    let mut bead = Issue::new(
        "bf-test".to_string(),
        "Test".to_string(),
        ".".to_string(),
    );
    bead.description = Some("Description".to_string());
    
    // Before creation
    println!("Before creation:");
    println!("  content_hash (field): {:?}", bead.content_hash);
    println!("  content_hash(): {}", bead.content_hash());
    println!("  json: {}", serde_json::to_string(&bead).unwrap());
    
    // Compute hash
    bead.content_hash = Some(bead.content_hash());
    
    println!("\nAfter setting content_hash:");
    println!("  content_hash (field): {:?}", bead.content_hash);
    println!("  json: {}", serde_json::to_string(&bead).unwrap());
}
