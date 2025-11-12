include!(concat!(env!("OUT_DIR"), "/coral_options.rs"));

fn main() {
    let options = Options::parse().expect("An error occurred parsing the arguments");
    println!("{options:#?}");
}
