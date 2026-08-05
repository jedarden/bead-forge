//! Test how None Option fields are serialized in TraceMetadata

use bead_forge::trace::TraceMetadata;

fn main() {
    let metadata = TraceMetadata {
        start_time: None,
        end_time: None,
        duration_ms: None,
        ..Default::default()
    };

    let json = serde_json::to_string_pretty(&metadata).unwrap();
    println!("Metadata with None timing fields:");
    println!("{}", json);

    // Check if the fields appear in the JSON
    if json.contains("\"start_time\"") {
        println!("\n✓ start_time field is present (as null)");
    } else {
        println!("\n✗ start_time field is NOT present");
    }
}
