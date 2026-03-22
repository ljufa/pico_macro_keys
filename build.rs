use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=.env");
    let dotenv_path = Path::new(".env");
    if dotenv_path.exists() {
        #[allow(deprecated)]
        for item in dotenv::from_path_iter(dotenv_path).unwrap() {
            let (key, val) = item.unwrap();
            println!("cargo:rustc-env={}={}", key, val);
        }
    }
}
