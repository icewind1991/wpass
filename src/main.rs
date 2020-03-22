use dirs::home_dir;
use main_error::MainError;
use rufi::MenuApp;
use std::collections::HashMap;
use std::env;
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

pub const WIN_W: u32 = 600;

fn main() -> Result<(), MainError> {
    let args: Vec<String> = env::args().skip(1).collect();
    let should_type = match args.first() {
        Some(arg) => arg == "--type",
        None => false,
    };

    let mut env: HashMap<String, String> = env::vars().collect();
    let prefix = env.remove("PASSWORD_STORE_DIR").unwrap_or_else(|| {
        let home = home_dir().expect("Failed to get home directory");
        let home = home.to_str().expect("Non utf8 home directory");
        format!("{}/.password-store/", home)
    });

    let files: Vec<String> = glob::glob(&format!("{}**/*.gpg", prefix))?
        .filter_map(|res| res.ok())
        .filter_map(|path| {
            path.to_str().map(|s| {
                s.trim_start_matches(&prefix)
                    .trim_end_matches(".gpg")
                    .to_string()
            })
        })
        .collect();

    let app = MenuApp::new(WIN_W, "WPass");

    let files = Arc::new(files);

    let item = {
        app.main_loop(move |query| {
            files
                .iter()
                .filter_map(|path| {
                    if path.contains(&query) {
                        Some(path.clone())
                    } else {
                        None
                    }
                })
                .take(15)
                .collect()
        })
    };

    let item = match item {
        Some(item) => item,
        None => {
            return Ok(());
        }
    };

    let mut command = Command::new("pass");

    if !should_type {
        command.arg("-c");
    }
    command.arg(&item);

    if should_type {
        let password = String::from_utf8(command.output()?.stdout)?;
        let password = password.trim_end_matches("\n");

        let path = "/var/run/evtype.sock";

        // give it some time to move focus back from the menu
        std::thread::sleep(Duration::from_millis(50));
        let mut stream = UnixStream::connect(path)?;
        stream.write_all(password.as_bytes())?;
    } else {
        command.spawn()?;
    }

    Ok(())
}
