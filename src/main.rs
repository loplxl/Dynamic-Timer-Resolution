use active_win_pos_rs::{get_active_window};
use rfd::FileDialog;
use std::{
    collections::HashMap,
    env,
    fs,
    io::{self, Write},
    thread,
    time::Duration,
    process::{
        Command,
        id
    }
};
use std::path::{Path, PathBuf};
use winapi;
use ntapi::ntexapi::NtSetTimerResolution;
use dirs;
use shortcuts_rs::ShellLink;

fn get_startup_dir() -> Result<PathBuf,&'static str> {
    if let Some(startup_dir) = dirs::config_dir() {
        let startup_dir = startup_dir.join("Microsoft\\Windows\\Start Menu\\Programs\\Startup");
        return Ok(startup_dir);
    }
    return Err("startup dir not found");
}
#[cfg(target_os = "windows")]
fn hide_console() {
    const SW_HIDE: i32 = 0;

    unsafe {
        let hwnd = winapi::um::wincon::GetConsoleWindow();
        if !hwnd.is_null() {
            winapi::um::winuser::ShowWindow(hwnd, SW_HIDE);
        }
    }
}
fn set_timer_resolution(resolution: u16, hold: bool) {
    let desired_time: winapi::shared::ntdef::ULONG = resolution as winapi::shared::ntdef::ULONG;
    
    let mut current_resolution: winapi::shared::ntdef::ULONG = 0; //random pointer to not get error
    unsafe {
        let exitcode = NtSetTimerResolution(
            desired_time,
            hold as u8,
            &mut current_resolution as *mut winapi::shared::ntdef::ULONG
        );
        if exitcode == 0 {
            if desired_time  != 0 {
                println!("Success! Set to {}\n",desired_time);
            } else {
                println!("Success! Reset timer resolution to be controlled by Windows.");
            }
            
        } else if exitcode != -1073741243 { println!("Failed to set timer resolution to {}. Error code {}\n",resolution,exitcode); }
        // -1073741243 means releasing hold on timer res
    }
}
fn clear_console() {
    let _ = std::process::Command::new("cmd")
        .args(&["/C", "cls"])
        .status();
}
fn sleep(ms: u64) {
    thread::sleep(Duration::from_millis(ms));
}
fn pause() {
    println!("Enter to continue.");
    io::stdin()
        .read_line(&mut String::new())
        .unwrap();
}
fn show_settings(settings_data: &mut str, settings_path: &str) {
    let mut settings_data = settings_data.to_string();
    loop {
        let mut choice: String = String::new();
        let mut settings_map = HashMap::new();
        for line in settings_data.lines() {
            if let Some((key, value)) = line.split_once(",") {
                settings_map.insert(key.trim(), value.trim());
            }
        }
        while !["1","3"].contains(&choice.trim()) {
            println!("└─┼───────────────┤ Settings");
            println!("1 │ Auto Startup  │ {}", settings_map.get("auto_startup").unwrap_or(&"false"));
            //println!("2 │ Scan Interval │ {}", settings_map.get("scan_interval").unwrap_or(&"1000"));
            println!("3 │ Save Settings │");
            choice.clear();
            print!(">> ");
            io::stdout().flush().unwrap(); //keep input on same line as prompt
            io::stdin()
                .read_line(&mut choice)
                .unwrap();
            clear_console();
        }
        choice = choice.trim().to_string();
        match choice.as_str() {
            "1" => {
                let mut create_shortcut: bool = false;
                if settings_data.contains("auto_startup,true") {
                    settings_data = settings_data.replace("auto_startup,true", "auto_startup,false");
                } else {
                    settings_data = settings_data.replace("auto_startup,false", "auto_startup,true");
                    create_shortcut = true;
                }
                let target: PathBuf = env::current_exe().expect("Failed to get current executable path for startup shortcut");
                let lnk: PathBuf = get_startup_dir().unwrap().join(format!("{}.lnk", target.file_name().unwrap().to_str().unwrap()));
                if create_shortcut {
                    let sl = ShellLink::new(target ,None ,None ,None ).unwrap();
                    sl.create_lnk(lnk).unwrap();
                } else { fs::remove_file(&lnk).expect("Failed to delete shortcut, maybe you forgot to save settings last time."); }
            }
            //"2" => {}
            "3" => {
                fs::write(&settings_path, &settings_data).unwrap();
                break;
            }
            _ => {}
        }
        clear_console();
    }
}
fn add_new(csv_str_dir: &str) {
    let csv_path = Path::new(&csv_str_dir);
    if let Some(parent) = csv_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    // create file if it doesnt exist
    fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(csv_path)
        .unwrap();
    clear_console();
    if let Some(path) = FileDialog::new()
        .set_title("Select a file")
        .pick_file()
    {
        let str_path: &str = path.to_str().unwrap();
        let exe_name: &str = str_path.split('\\').last().unwrap();
        println!("Selected: {}", path.display());
        let csv_contents: String = fs::read_to_string(csv_str_dir).unwrap_or_default();
        let mut data_to_write: Vec<String> = Vec::new();
        //overwrite logic
        if csv_contents.contains(str_path) {
            loop {
                println!("A timer resolution for {} already exists. Overwrite? (y/n)", exe_name);
                let mut choice: String = String::new();
                io::stdin()
                    .read_line(&mut choice)
                    .unwrap();
                choice = choice.trim().to_lowercase();
                if choice == "y" || choice == "yes" {
                    let mut lines: Vec<&str> = csv_contents.lines().collect::<Vec<&str>>();
                    for line in lines.iter_mut() {
                        if !(line.contains(str_path)) {
                            data_to_write.push(line.to_string());
                        }
                    }
                    clear_console();
                    break;
                } else if choice == "n" || choice == "no" {
                    return;
                }
            }
        } else {
            data_to_write.push(csv_contents);
        }
        loop {
            let mut resolution = String::new();
            println!("What resolution?");
            print!(">> ");
            io::stdout().flush().unwrap(); //keep input on same line as prompt
            io::stdin()
                .read_line(&mut resolution)
                .unwrap();
            resolution = resolution.trim().to_string();
            let parsed_resolution: u16 = match resolution.parse::<u16>() {
                Ok(v) => v,
                Err(_e) => {
                    println!("Type Error: Please use an integer from 5000-15625");
                    return;
                }
            };
            if !(5000..15625).contains(&parsed_resolution) {
                println!("Bound error: Please use an integer from 5000-15625");
            } else {
                data_to_write.push(format!("{},{}", str_path, parsed_resolution));
                fs::write(csv_path,data_to_write.join("\n")).unwrap();
                break;
            }
        }
        Command::new(env::current_exe().expect("Failed to get current executable path"))
            .spawn()
            .expect("Failed to restart the program");
    }
}

fn main() {

    let pid: String = id().to_string();
    let tasklist =  Command::new("tasklist")
        .arg("/FI")
        .arg("IMAGENAME eq dynamic-timer-resolution.exe")
        .args(["/FO","CSV"])
        .output()
        .expect("Failed to list active timer dynamic timer resolution exes");
    let stdout = String::from_utf8_lossy(&tasklist.stdout);
    for line in stdout.lines() {
        let parts: Vec<&str> = line.split(',').collect();
        let cur_pid = parts[1].trim_matches('"');
        if cur_pid != pid && cur_pid != "PID" {
            Command::new("taskkill")
                .args(["/f","/pid",&format!("{}",cur_pid)])
                .status()
                .expect("Failed to kill task");
            println!("Killed hanging process with PID {}\n",cur_pid)
        }
    }
    let trh_dir: String = format!("{}\\dynamic-timer-resolution",env::var("LOCALAPPDATA").unwrap());
    fs::create_dir_all(&trh_dir).expect("Failed to create TRH dir");
    let csv_dir: String = format!("{}\\data.csv",trh_dir);


    let settings_path: String = format!("{}\\settings.csv", trh_dir);
    let mut settings_file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(&settings_path)
        .unwrap();
    let mut settings_data: String = fs::read_to_string(&settings_path).unwrap_or_default();
    if settings_data == "" {
        settings_data = "auto_startup,false\n\
            scan_interval,1000".to_owned();
        settings_file.write(&settings_data.as_bytes()).unwrap();
    }

    let mut choice: String = String::new();
    while !["1","2","3","9","0"].contains(&choice.trim()) {
        clear_console();
        println!("Welcome to loplxl's Dynamic Timer Resolution tool!");
        println!("1 > Continue to program");
        println!("2 > Continue to program with no console (requires to be ended with task manager)");
        println!("3 > Add new program");
        println!("9 > Change settings");
        choice.clear();
        print!(">> ");
        io::stdout().flush().unwrap(); //keep input on same line as prompt
        io::stdin()
            .read_line(&mut choice)
            .unwrap();
        clear_console();
    }
    choice = choice.trim().to_string();
    match choice.as_str() {
        "2" => {
            hide_console();
        },
        "3" => {
            add_new(&csv_dir);
        }
        "9" => {
            show_settings(&mut settings_data,&settings_path);
            main();
        }
        "0" => {return;}
        _ => {}
    }   
    
    Command::new("taskkill")
        .args(["/f","/im","SetTimerResolution.exe"])
        .spawn()
        .expect("Failed to kill SetTimerResolution.exe (if it exists)");

    let mut resolution_lookup: HashMap<String, u16> = HashMap::new(); //speed yeeeees
    //add saved values to a hashmap for faster lookup
    if fs::exists(&csv_dir).expect("Failed to check if CSV exists") {
        let data: String = fs::read_to_string(csv_dir).expect("Failed to read CSV").trim().to_string();
        for line in data.lines() {
            if line.contains(",") {
                if let Some((executable,resolution)) = line.split_once(",") {
                    resolution_lookup.insert(executable.to_string(),resolution.parse::<u16>().expect("Failed to parse resolution as integer"));
                    println!("Adding <{executable}> to list with resolution <{resolution}>");
                }
            }
        }
    }
    println!();
    let mut current_resolution: u16 = 0;
    let mut last_window: String = String::new();
    loop {
        if let Ok(window) = get_active_window() {
            let process = window.process_path.to_str().unwrap().to_string();
            if process != last_window {
                last_window = process.clone();
                println!("Detected active window <{}>",process);
                let desired_resolution: u16 = resolution_lookup
                    .get(&last_window)
                    .copied()
                    .unwrap_or(0);
                if desired_resolution != current_resolution {
                    if desired_resolution == 0 {
                        println!("Detected swap to {}\nResetting resolution...",process);
                    } else {
                        println!("Detected swap to {}\nSetting resolution to {}",process,desired_resolution);
                    }
                    current_resolution = desired_resolution; 
                    set_timer_resolution(desired_resolution,desired_resolution != 0);
                }
            } else { sleep(16) }
            sleep(1000)
        } else { sleep(16) }
    }
}

