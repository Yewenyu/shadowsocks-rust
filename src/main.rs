use std::{env, sync::mpsc, thread, time::Duration};

use shadowsocks_rust::service::local;

fn main() {
    let base_dir = env::current_dir().expect("not found path");

    let configPath = String::from(base_dir.to_str().expect("msg")) + "/src/service/test/config.json";
    println!("{}", configPath.as_str());
    local::start(configPath.as_str(), false, |ts| {
        let _ = thread::spawn(move || {
            thread::sleep(Duration::from_millis(10000));
            ts.send(true);
        });
    });

    local::start(configPath.clone().as_str(), true, |ts| {
        let _ = thread::spawn(move || {
            thread::sleep(Duration::from_millis(20000));
            ts.send(true);
        });
    });

    println!("");
    thread::sleep(Duration::from_secs(1000000));
}
