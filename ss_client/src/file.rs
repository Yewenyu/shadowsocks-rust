use std::fs;

pub fn get_content(path: String) -> String {
    let content = fs::read_to_string(path).unwrap();
    return content;
}
pub fn writeFile(path: String, content: String) {
    fs::write(path, content).unwrap();
}
