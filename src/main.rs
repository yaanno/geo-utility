use geo_utility::generation::generate::generate_synthetic_data;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = generate_synthetic_data();
    Ok(())
}
