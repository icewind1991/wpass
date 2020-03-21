use main_error::MainError;
use std::io::Write;
use std::os::unix::net::UnixStream;
use rufi::{MenuApp, Renderer, EventsLoop};
use std::time::Duration;
use std::env;
use std::collections::HashMap;
use dirs::home_dir;
use std::path::PathBuf;
use std::process::Command;

pub const WIN_W: u32 = 600;

#[tokio::main]
async fn main() -> Result<(), MainError> {
    let args: Vec<String> = env::args().skip(1).collect();
    let should_type = match args.first() {
        Some(arg) => arg == "--type",
        None => false
    };

    let mut env: HashMap<String, String> = env::vars().collect();
    let prefix = env.remove("PASSWORD_STORE_DIR").unwrap_or_else(|| {
        let home = home_dir().expect("Failed to get home directory");
        let home = home.to_str().expect("Non utf8 home directory");
        format!("{}/.password-store/", home)
    });

    let files: Vec<PathBuf> = glob::glob(&format!("{}**/*.gpg", prefix))?.collect::<Result<Vec<PathBuf>, _>>()?;

    let events_loop = EventsLoop::new();

    let renderer = Renderer::new(&events_loop, "WPass", WIN_W);

    let app = MenuApp::new(WIN_W, events_loop);

    let item = {
        app.main_loop(renderer, move |query| {
            let result = files.iter().filter_map(|path| {
                let path_str = path.as_os_str().to_str().unwrap_or_default();
                if path_str.contains(&query) {
                    Some(path_str.trim_start_matches(&prefix).trim_end_matches(".gpg").to_string())
                } else {
                    None
                }
            }).collect();
            async move {
                tokio::time::delay_for(Duration::from_millis(100)).await; // debounce
                result
            }
        })
            .await
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
