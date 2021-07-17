use anyhow::{
    Result,
    anyhow,
    bail
};

use std::{
    env,
    fs,
    time::Duration,
    path::PathBuf,
    process::Command,
    collections::HashMap
};

use notify::{watcher, RecursiveMode, Watcher, DebouncedEvent};

fn main() -> Result<()> {
    let path_map = env::var("PATH_MAP")
        .map_err(|_| anyhow!("No map"))
        .and_then(|wm| serde_json::from_str::<HashMap<String, String>>(&wm)
            .map_err(|e| anyhow!("Cannot deserialize path map: {}", e)))?;

    import_existing(&path_map);
    Ok(watch(&path_map)?)
}

fn import_existing(path_map: &HashMap<String, String>) {
    for (source_path,_) in path_map {
        match fs::read_dir(source_path) {
            Ok(files) => {
                for file in files {
                    if let Err(e) = file.map_err(|e| anyhow!("Invalid file: {}", e))
                        .and_then(|file| handle_file(file.path(), path_map))
                    {
                        println!("Could not import file: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("Could not read dir: {}", e);
            }
        }
    }
}

fn handle_file(source_file: PathBuf, path_map: &HashMap<String, String>) -> Result<()> {
    let ext = source_file.extension()
        .and_then(|ext| ext.to_str())
        .ok_or(anyhow!("Cannot handle file without extension"))?;

    if ext.to_lowercase() != "pdf" {
        bail!("Currently only pdf files are supported");
    }

    let source_path = source_file.parent()
        .and_then(|p| p.to_str())
        .ok_or(anyhow!("No path for source"))?;

    let file_name = source_file.file_name()
        .and_then(|f| f.to_str()).ok_or(anyhow!("No filename for source"))?;

    let target_file = path_map.get(source_path)
        .map(|t| PathBuf::from(t))
        .map(|p| p.join(file_name))
        .ok_or(anyhow!("No target for source"))?;

    let status = Command::new("ocrmypdf")
        .arg(&source_file)
        .arg(target_file)
        .spawn()?
        .wait()?;

    if !status.success() {
        bail!("Handle file failed with exit code: {}", status);
    }

    fs::remove_file(source_file)?;
    Ok(())
}

fn watch(path_map: &HashMap<String, String>) -> Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut w = watcher(tx, Duration::from_secs(1))?;

    for (src, _) in path_map.iter() {
        if let Err(e) = w.watch(src, RecursiveMode::NonRecursive) {
            eprintln!("Cannot watch path '{}': {}", src, e);
        }
    }

    loop {
        match rx.recv() {
            Ok(r) => {
                match r {
                    DebouncedEvent::Create(path) => {
                        if let Err(e) = handle_file(path, &path_map) {
                            eprintln!("{}", e);
                        }
                    }
                    _ => ()
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }
}
