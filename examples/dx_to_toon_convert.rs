use std::fs;
use serializer::converters::toon::dx_to_toon;

fn main() {
    let dx_llm = fs::read_to_string("sample-output/sample-json.llm").expect("read .llm");
    let toon = dx_to_toon(&dx_llm).expect("convert to toon");
    fs::write("sample-output/sample-json.toon", &toon).expect("write .toon");
    println!("{}", toon);
}
