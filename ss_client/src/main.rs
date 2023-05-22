use std::{env, thread, time::Duration};

fn main() {
    let base_dir = env::current_dir().expect("not found path");

    let configPath = String::from(base_dir.to_str().expect("msg")) + "/ss_client/src/config.json";
    println!("{}", configPath);
    // let newP = configPath.clone();
    // let _ = thread::spawn(move || {
    //     thread::sleep(Duration::from_millis(10000));
    //     ss_client::ss_stop();
    // });
    ss_client::ss_start(configPath);

    print!("Hello, world!")
    // let newPP = newP.clone();
    // let _ = thread::spawn(move || {
    //     thread::sleep(Duration::from_millis(10000));
    //     ss_client::ss_stop();
    // });
    // ss_client::ss_start(newP, true);
    // let _ = thread::spawn(move || {
    //     thread::sleep(Duration::from_millis(10000));
    //     ss_client::ss_stop();
    // });
    // ss_client::ss_start(newPP, true);
}
