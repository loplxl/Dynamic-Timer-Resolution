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
use winapi::{
    shared::ntdef::{ULONG},
    um
};
use ntapi::ntexapi::NtSetTimerResolution;
#[cfg(target_os = "windows")]
fn hide_console() {
    const SW_HIDE: i32 = 0;

    unsafe {
        let hwnd = um::wincon::GetConsoleWindow();
        if !hwnd.is_null() {
            um::winuser::ShowWindow(hwnd, SW_HIDE);
        }
    }
}


fn set_timer_resolution(resolution: u16, hold: bool) {
    let desired_time: ULONG = resolution as ULONG;
    
    let mut current_resolution: ULONG = 0; //random pointer to not get error
    unsafe {
        let exitcode = NtSetTimerResolution(
            desired_time,
            hold as u8,
            &mut current_resolution as *mut ULONG
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

fn sleep(ms: u64) {
    thread::sleep(Duration::from_millis(ms));
}
fn pause() {
    println!("Enter to continue.");
    io::stdin()
        .read_line(&mut String::new())
        .unwrap();
}
fn addnew(csv_dir: &String) {
    if let Some(path) = FileDialog::new()
        .set_title("Select a file")
        .pick_file()
    {
        let mut resolution = String::new();
        loop {
            print!("\x1B[2J\x1B[1;1H"); //clear console
            println!("What resolution for {}?",path.display());
            resolution.clear();
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
                return;
            }
            let mut file = fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(csv_dir).expect("Failed to create CSV.");
            writeln!(file,"{},{}\n",path.display(),resolution).expect("Failed to write to data.csv");
            return;
        }
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
    let mut choice: String = String::new();
    while !["1","2","9"].contains(&choice.trim()) {
        println!("Welcome to loplxl's Dynamic Timer Resolution tool!");
        println!("1 > Continue to program");
        println!("2 > Add new program");
        println!("9 > Continue to program with no console (requires to be ended with task manager)");
        choice.clear();
        print!(">> ");
        io::stdout().flush().unwrap(); //keep input on same line as prompt
        io::stdin()
            .read_line(&mut choice)
            .unwrap();
        print!("\x1B[2J\x1B[1;1H"); //clear console
    }
    choice = choice.trim().to_string();
    match choice.as_str() {
        "2" => {
            addnew(&csv_dir);
            choice.clear();
            pause();
            main();
            return;
        },
        "9" => {
            hide_console();
        }
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