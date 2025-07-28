// External Crate Imports
use eyre::Result;
use quickxml_to_serde::{xml_string_to_json, Config};
use regex::Regex;
use reqwest;
use serde_json::{json, Map, Value};
use zip::ZipArchive;

// Standard Library Imports
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub fn count_xml_files(dir: &Path) -> eyre::Result<usize> {
    let entries = fs::read_dir(dir)?;
    let mut count = 0;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            count += count_xml_files(&path)?;
        } else if path.is_file()
            && path.extension().map_or(false, |ext| ext == "xml")
            && !path.to_string_lossy().ends_with("-org.xml")
        {
            count += 1;
        }
    }

    Ok(count)
}

pub fn try_convert_xml_to_json(xml: &str, config: &Config) -> Result<String, eyre::Report> {
    let json = xml_string_to_json((&xml).to_string(), config)?;
    Ok(serde_json::to_string(&json)?)
}

pub fn preprocess_xml(input: &str) -> String {
    let mut processed_xml = input.to_string();

    // Remove URLs encoded see gef-mx example
    let re = Regex::new(r"<https?://[^>]*>").unwrap();
    processed_xml = re.replace_all(&processed_xml, "").to_string();

    return processed_xml;
}

fn is_human_text(s: &str) -> bool {
    let t = s.trim();
    if t.len() < 4 || !t.contains(' ') {
        return false;
    }
    t.chars().any(|c| c.is_alphabetic())
}

fn collect_narratives(v: &Value, bag: &mut HashSet<String>) {
    match v {
        Value::Object(o) => {
            for (k, v2) in o {
                if k == "narrative" {
                    if let Value::String(s) = v2 {
                        if is_human_text(s) {
                            bag.insert(s.trim().to_owned());
                        }
                    }
                }
                collect_narratives(v2, bag); // descend
            }
        }
        Value::Array(a) => a.iter().for_each(|v2| collect_narratives(v2, bag)),
        _ => {}
    }
}

pub fn filter_activity(activity: &Value) -> Value {
    const KEEP: [&str; 12] = [
        "default-currency",
        "last-updated-datetime",
        "iati-identifier",
        "reporting-org",
        "title",
        "description",
        "participating-org",
        "activity-status",
        "recipient-country",
        "recipient-region",
        "location",
        "transaction",
    ];
    let wanted: HashSet<_> = KEEP.iter().cloned().collect();
    let obj = match activity.as_object() {
        Some(o) => o,
        None => return Value::Object(Map::new()),
    };

    //---------------- meta -----------------
    let mut meta = Map::new();

    // sectors (all) + main purpose
    if let Some(sec_val) = obj.get("sector") {
        let src = match sec_val {
            Value::Array(a) => a.clone(),
            Value::Object(o) => vec![Value::Object(o.clone())],
            _ => vec![],
        };

        // build sector list
        let mut sectors: Vec<Map<String, Value>> = src
            .into_iter()
            .filter_map(|s| {
                let o = s.as_object()?;
                let mut item = Map::new();
                item.insert("dac_code".into(), o.get("code")?.clone());
                if let Some(n) = o.get("narrative") {
                    item.insert("name".into(), n.clone());
                }
                if let Some(p) = o.get("percentage") {
                    item.insert("percentage".into(), p.clone());
                }
                Some(item)
            })
            .collect();

        if !sectors.is_empty() {
            // select purpose = highest percentage or first
            sectors.sort_by(|a, b| {
                let pa = a.get("percentage").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let pb = b.get("percentage").and_then(|v| v.as_f64()).unwrap_or(0.0);
                pb.partial_cmp(&pa).unwrap() // descending
            });
            meta.insert(
                "sectors".into(),
                Value::Array(sectors.iter().cloned().map(Value::Object).collect()),
            );
            meta.insert("purpose".into(), Value::Object(sectors[0].clone()));
        }
    }

    // policy‑markers
    if let Some(pm) = obj.get("policy-marker") {
        let src = match pm {
            Value::Array(a) => a.clone(),
            Value::Object(o) => vec![Value::Object(o.clone())],
            _ => vec![],
        };
        let mks: Vec<Value> = src
            .into_iter()
            .filter_map(|v| {
                let m = v.as_object()?;
                let mut out = Map::new();
                if let Some(c) = m.get("code") {
                    out.insert("code".into(), c.clone());
                }
                if let Some(sig) = m.get("significance") {
                    out.insert("significance".into(), sig.clone());
                }
                if let Some(n) = m.get("narrative") {
                    out.insert("name".into(), n.clone());
                }
                (!out.is_empty()).then(|| Value::Object(out))
            })
            .collect();
        if !mks.is_empty() {
            meta.insert("policy_markers".into(), Value::Array(mks));
        }
    }

    // single‑code helpers
    let grab = |key: &str| -> Option<Value> {
        obj.get(key)
            .or_else(|| obj.get(&format!("default-{}", key)))
            .and_then(|v| v.get("code"))
            .cloned()
    };
    if let Some(v) = grab("aid-type") {
        meta.insert("aid_type".into(), json!({"code":v}));
    }
    if let Some(v) = grab("finance-type") {
        meta.insert("finance_type".into(), json!({"code":v}));
    }

    // related activities
    if let Some(rel) = obj.get("related-activity") {
        let src = match rel {
            Value::Array(a) => a.clone(),
            Value::Object(o) => vec![Value::Object(o.clone())],
            _ => vec![],
        };
        let rels: Vec<Value> = src
            .into_iter()
            .filter_map(|v| {
                let o = v.as_object()?;
                let mut m = Map::new();
                if let Some(r) = o.get("ref") {
                    m.insert("ref".into(), r.clone());
                }
                if let Some(t) = o.get("type") {
                    m.insert("type".into(), t.clone());
                }
                (!m.is_empty()).then(|| Value::Object(m))
            })
            .collect();
        if !rels.is_empty() {
            meta.insert("related_activities".into(), Value::Array(rels));
        }
    }

    //---------------- core fields ----------------
    let mut out = Map::new();
    for (k, v) in obj {
        if !wanted.contains(k.as_str()) {
            continue;
        }
        match k.as_str() {
            "description" => {
                let arr = match v {
                    Value::Array(a) => a.clone(),
                    Value::Object(o) => vec![Value::Object(o.clone())],
                    _ => vec![],
                };
                out.insert(k.clone(), Value::Array(arr));
            }
            "transaction" => {
                let src = match v {
                    Value::Array(a) => a.clone(),
                    Value::Object(o) => vec![Value::Object(o.clone())],
                    _ => vec![],
                };
                let txs: Vec<Value> = src
                    .into_iter()
                    .filter_map(|t| {
                        let m = t.as_object()?;
                        let tt = m.get("transaction-type")?.get("code")?;
                        let td = m.get("transaction-date")?.get("iso-date")?;
                        let val = m.get("value")?;
                        let tv = val.get("#text").or_else(|| val.get("value"))?;
                        let mut n = Map::new();
                        n.insert("transaction-type".into(), tt.clone());
                        n.insert("transaction-date".into(), td.clone());
                        n.insert("transaction-value".into(), tv.clone());
                        if let Some(c) = val.get("currency") {
                            n.insert("transaction-currency".into(), c.clone());
                        }
                        Some(Value::Object(n))
                    })
                    .collect();
                out.insert(k.clone(), Value::Array(txs));
            }
            "title" => {
                if let Some(narr) = v.get("narrative") {
                    out.insert(k.clone(), narr.clone());
                }
            }
            "reporting-org" => {
                let mut ro = Map::new();
                if let Some(id) = v.get("ref") {
                    ro.insert("id".into(), id.clone());
                }
                if let Some(t) = v.get("type") {
                    ro.insert("type".into(), t.clone());
                }
                if let Some(n) = v.get("narrative") {
                    ro.insert("name".into(), n.clone());
                }
                out.insert(k.clone(), Value::Object(ro));
            }
            _ => {
                out.insert(k.clone(), v.clone());
            }
        }
    }
    if !meta.is_empty() {
        out.insert("meta".into(), Value::Object(meta));
    }

    let mut bag = HashSet::new();
    collect_narratives(activity, &mut bag); // use *original* activity
    let free_text = bag.into_iter().collect::<Vec<_>>().join(" ");

    if !free_text.is_empty() {
        let meta_val = out
            .entry("meta".to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        if let Value::Object(m) = meta_val {
            m.insert("results".to_string(), Value::String(free_text));
        }
    }

    Value::Object(out)
}

pub fn download_zip(url: &str) -> eyre::Result<PathBuf> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(900))
        .build()?;

    // Fetch the content from the URL
    let response = client.get(url).send()?;

    if !response.status().is_success() {
        return Err(eyre::eyre!("Failed to fetch the ZIP file from the URL."));
    }

    let bytes = response.bytes()?;

    // Create the 'raw_data' directory if it doesn't exist
    let dir_path = PathBuf::from("raw_data");
    if !dir_path.exists() {
        fs::create_dir(&dir_path)?;
    }

    let zip_path = dir_path.join("mock_iati_data.zip");
    fs::write(&zip_path, bytes)?;

    Ok(zip_path)
}

pub fn extract_zip(zip_path: &Path) -> eyre::Result<String> {
    let file = File::open(&zip_path)?;
    let mut archive = ZipArchive::new(file)?;

    let extraction_path = PathBuf::from("raw_data");
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let outpath = extraction_path.join(file.name());

        if file.name().ends_with('/') {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut outfile = File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
    }
    Ok(extraction_path.to_string_lossy().into_owned())
}

// used in case of dynamically named zipped folders
pub fn find_data_folder(base_path: &Path) -> eyre::Result<Option<PathBuf>> {
    for entry in fs::read_dir(base_path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().unwrap() == "data" {
                return Ok(Some(path));
            } else {
                if let Some(data_path) = find_data_folder(&path)? {
                    return Ok(Some(data_path));
                }
            }
        }
    }
    Ok(None)
}

pub fn process_root(zip_path: &Path) -> eyre::Result<String> {
    let extraction_base_path = extract_zip(&zip_path)?;

    let mut found_root_path = None;

    // Iterate over directories inside extraction_base_path
    for entry in std::fs::read_dir(&extraction_base_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() && path.join("metadata.json").exists() && path.join("data").exists() {
            found_root_path = Some(path);
            break;
        }
    }

    let root_path = match found_root_path {
        Some(p) => p,
        None => {
            return Err(eyre::eyre!(
                "Couldn't locate the root directory containing metadata.json and data folder."
            ))
        }
    };

    // Read and pretty print metadata.json content
    let metadata_path = root_path.join("metadata.json");
    let mut file = File::open(metadata_path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    // Pretty print the JSON content
    let json_content: Value = serde_json::from_str(&content)?;
    let pretty_json = serde_json::to_string_pretty(&json_content)?;

    println!(
        "\n============== Metadata for Content ==============\n\n{}\n\n===================================================\n",
        pretty_json
    );

    // Read and process errors.txt
    let errors_txt_path = root_path.join("errors.txt");
    if errors_txt_path.exists() {
        let errors_file = File::open(&errors_txt_path)
            .map_err(|e| eyre::eyre!("Failed to open {}: {}", errors_txt_path.display(), e))?;
        let reader = BufReader::new(errors_file);

        // Collect all lines into a vector
        let lines: Vec<String> = reader
            .lines()
            .collect::<Result<_, io::Error>>()
            .map_err(|e| {
                eyre::eyre!(
                    "Failed to read lines from {}: {}",
                    errors_txt_path.display(),
                    e
                )
            })?;

        // Take the top 5 errors
        let top_five = lines.into_iter().take(5).collect::<Vec<String>>();

        // Print the top 5 errors
        println!("\n================ Top 5 Errors =================\n");
        for (i, error) in top_five.iter().enumerate() {
            println!("{}. {}", i + 1, error);
        }
        println!("\n===================================================\n");
    }
    // Return the path to the `data` folder
    Ok(root_path.join("data").to_string_lossy().into_owned())
}

/// Represents an entry in errors.txt
struct ErrorEntry {
    folder_title: String,
    file_name: String,
    url: String, // Include if needed
}

pub fn supplement_root_files() -> Result<String> {
    // Define the path to errors.txt
    let errors_txt_path = Path::new("./raw_data/iati-data-main/errors.txt");

    if !errors_txt_path.exists() {
        println!("\n================ No Errors Found =================\n");
        return Ok("No errors.txt file found.".to_owned());
    }

    // Open errors.txt
    let errors_file = File::open(&errors_txt_path)
        .map_err(|e| eyre::eyre!("Failed to open {}: {}", errors_txt_path.display(), e))?;
    let reader = BufReader::new(errors_file);

    // Collect all lines into a vector
    let lines: Vec<String> = reader
        .lines()
        .collect::<Result<_, io::Error>>()
        .map_err(|e| {
            eyre::eyre!(
                "Failed to read lines from {}: {}",
                errors_txt_path.display(),
                e
            )
        })?;

    // Check if there are any lines
    if lines.is_empty() {
        println!("\n================ No Errors Found =================\n");
        return Ok("errors.txt is empty.".to_owned());
    }

    // Parse lines into ErrorEntry structs
    let mut error_entries = Vec::new();
    let mut malformed_lines = 0;

    for (i, line) in lines.iter().enumerate() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            eprintln!("Line {} is malformed: {}", i + 1, line);
            malformed_lines += 1;
            continue;
        }

        let folder_title = parts[1].to_string();
        let file_name = parts[2].to_string();
        let url = parts[3].to_string(); // Use if needed

        error_entries.push(ErrorEntry {
            folder_title,
            file_name,
            url,
        });
    }

    let total_lines = lines.len();

    // Define source and destination base paths
    // let source_base = Path::new("./raw_data/iati-data-main/data"); // Raw data directory: ./raw_data/iati-data-main/data
    let archive_source_base = Path::new("./archive/iati-data-main/data"); // Archive directory: ./archive/iati-data-main/data
    let destination_base = Path::new("./raw_data/iati-data-main/data"); // Destination is the raw data directory

    // Initialize storage for files to copy
    let mut files_to_copy: HashMap<String, Vec<String>> = HashMap::new();

    // Iterate through each ErrorEntry to identify missing or empty files
    for entry in &error_entries {
        // Ensure the file name ends with .xml
        let file_name = if entry.file_name.ends_with(".xml") {
            entry.file_name.clone()
        } else {
            format!("{}.xml", entry.file_name)
        };

        // Construct the expected file path in the raw data directory
        let destination_file_path = destination_base.join(&entry.folder_title).join(&file_name);

        let mut needs_copy = false;

        if !destination_file_path.exists() || !destination_file_path.is_file() {
            // File does not exist or is not a regular file
            needs_copy = true;
        } else {
            // File exists; check if it's empty
            match fs::metadata(&destination_file_path) {
                Ok(metadata) => {
                    if metadata.len() == 0 {
                        println!(
                            "⚠️ File '{}' is empty. It will be overwritten.",
                            destination_file_path.display()
                        );
                        needs_copy = true;
                    }
                }
                Err(e) => {
                    eprintln!(
                        "❌ Failed to retrieve metadata for '{}': {}",
                        destination_file_path.display(),
                        e
                    );
                    // Decide whether to treat this as needing copy or skip
                    needs_copy = true; // Assuming we want to attempt to copy
                }
            }
        }

        if needs_copy {
            files_to_copy
                .entry(entry.folder_title.clone())
                .or_insert_with(Vec::new)
                .push(file_name.clone());
        }
    }

    // If there are no files to copy, inform the user and exit
    if files_to_copy.is_empty() {
        println!("\n================ No Missing or Empty Files to Copy =================\n");
        return Ok("All files are present and non-empty. No copies needed.".to_owned());
    }

    // Perform copying operations
    println!("\n===================== Copy Operations =====================\n");

    for (folder_title, file_names) in files_to_copy.iter() {
        let destination_folder = destination_base.join(folder_title);
        let archive_folder = archive_source_base.join(folder_title); // Using archive as the source

        // Ensure the destination folder exists
        if !destination_folder.exists() {
            println!(
                "📁 Destination folder '{}' does not exist. Creating...",
                destination_folder.display()
            );

            match fs::create_dir_all(&destination_folder) {
                Ok(_) => {
                    println!(
                        "✅ Created destination directory '{}'",
                        destination_folder.display()
                    );
                }
                Err(e) => {
                    eprintln!(
                        "❌ Failed to create destination directory '{}': {}",
                        destination_folder.display(),
                        e
                    );
                    continue; // Skip copying files in this folder
                }
            }
        }

        // Iterate through each file and perform the copy from archive
        for file_name in file_names {
            let source_file_path = archive_folder.join(&file_name); // Source is archive
            let destination_file_path = destination_folder.join(&file_name);

            // Attempt to copy the file
            match fs::copy(&source_file_path, &destination_file_path) {
                Ok(bytes) => {
                    println!(
                        "📄 Copied file '{}' ({} bytes) to '{}'",
                        file_name,
                        bytes,
                        destination_file_path.display()
                    );

                    // Optional: Verify that the copied file matches the source
                    let source_metadata = fs::metadata(&source_file_path)?;
                    let destination_metadata = fs::metadata(&destination_file_path)?;

                    if source_metadata.len() == destination_metadata.len() {
                        println!("✅ Verified that '{}' was copied correctly.", file_name);
                    } else {
                        eprintln!(
                            "⚠️ Size mismatch for '{}'. Source: {} bytes, Destination: {} bytes.",
                            file_name,
                            source_metadata.len(),
                            destination_metadata.len()
                        );
                    }
                }
                Err(e) => {
                    eprintln!(
                        "❌ Failed to copy file '{}' to '{}': {}",
                        source_file_path.display(),
                        destination_file_path.display(),
                        e
                    );
                }
            }
        }
    }

    println!("\n==============================================================\n");

    // Create a summary message
    let summary = format!(
        "Checked {} files with {} malformed lines.\nFiles to copy: {}.\nCopy operations completed.",
        total_lines,
        malformed_lines,
        files_to_copy.values().map(|v| v.len()).sum::<usize>()
    );

    Ok(summary)
}
