use std::{env, thread, time::Duration};

fn main() {
    let base_dir = env::current_dir().expect("not found path");

    let configPath = String::from(base_dir.to_str().expect("msg")) + "/ss_client/src/config.son";
    println!("{}", configPath);
    // let newP = configPath.clone();
    // let _ = thread::spawn(move || {
    //     thread::sleep(Duration::from_millis(10000));
    //     ss_client::ss_stop();
    // });
    let code = ss_client::ss_start("/Users/ye/Library/Containers/840178FC-DF61-49B9-B1BA-08765CE6AC65/Data/Documents/SSRustConfig.json".to_string());

    println!("ss_client exited with code: {}", code);
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
