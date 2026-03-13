

use std::process::ExitCode;

use clap::Command;
use shadowsocks_rust::service::local;


pub fn ss_start(path: String) -> i32 {

    let mut app = Command::new("shadowsocks")
        .version(shadowsocks_rust::VERSION)
        .about("A fast tunnel proxy that helps you bypass firewalls. (https://shadowsocks.org)");
    app = local::define_command_line_options(app);


    // 模拟从外部接收的命令行参数
    let external_args = vec!["sslocal", "-c", path.as_str()];

    // 使用模拟的外部参数解析命令行
    let matches = app.get_matches_from(external_args);
     match local::create(&matches).and_then(|(runtime, main_fut)| runtime.block_on(main_fut)) {
        Ok(()) => return 0,
        Err(err) => {
            eprintln!("{err}");
            return err.exit_code() as i32;
        }
    }
    
    
}
