// External Crate Imports
use eyre::Result;
use quickxml_to_serde::{xml_string_to_json, Config};
use regex::Regex;
use reqwest;
use serde_json::Value;
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

use serde_json::Map;

pub fn filter_activity(activity: &Value) -> Value {
    const DESIRED_KEYS: [&str; 12] = [
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

    let desired_keys = DESIRED_KEYS.iter().cloned().collect::<HashSet<_>>();

    let mut filtered_activity = Map::new();

    let obj = match activity.as_object() {
        Some(o) => o,
        None => return Value::Object(filtered_activity),
    };

    for (key, value) in obj {
        if !desired_keys.contains(key.as_str()) {
            continue;
        }

        match key.as_str() {
            "description" => {
                let descriptions = match value {
                    Value::Array(arr) => arr.clone(),
                    Value::Object(obj) => vec![Value::Object(obj.clone())],
                    _ => vec![], // Default to an empty array
                };
                filtered_activity.insert(key.clone(), Value::Array(descriptions));
            }

            "transaction" => {
                let transactions = match value {
                    Value::Array(arr) => arr.clone(),
                    Value::Object(obj) => vec![Value::Object(obj.clone())],
                    _ => continue,
                };

                let transformed_transactions: Vec<Value> = transactions
                    .into_iter()
                    .filter_map(|transaction| {
                        let mut new_transaction = Map::new();

                        // **Essential Field: "transaction-type" -> "code"**
                        let transaction_type = transaction
                            .get("transaction-type")
                            .and_then(|tt| tt.get("code"))
                            .cloned();
                        if let Some(tt) = transaction_type {
                            new_transaction.insert("transaction-type".to_string(), tt);
                        } else {
                            // Skip this transaction if "transaction-type" is missing
                            return None;
                        }

                        // **Essential Field: "transaction-date" -> "iso-date"**
                        let transaction_date = transaction
                            .get("transaction-date")
                            .and_then(|td| td.get("iso-date"))
                            .cloned();
                        if let Some(td) = transaction_date {
                            new_transaction.insert("transaction-date".to_string(), td);
                        } else {
                            // Skip this transaction if "transaction-date" is missing
                            return None;
                        }

                        // **Essential Field: "value" -> Text Content**
                        if let Some(value_obj) = transaction.get("value") {
                            // Attempt to parse the numerical value
                            let transaction_value = value_obj
                                .get("#text")
                                .or_else(|| value_obj.get("value"))
                                .cloned();

                            // Ensure that the transaction value exists and is a number
                            if let Some(tv) = transaction_value {
                                new_transaction.insert("transaction-value".to_string(), tv);
                            } else {
                                // Skip this transaction if "transaction-value" is missing
                                return None;
                            }

                            // **Optional Field: "currency"**
                            if let Some(currency) = value_obj.get("currency").cloned() {
                                new_transaction
                                    .insert("transaction-currency".to_string(), currency);
                            }
                            // If "currency" is missing, omit the "transaction-currency" field
                        } else {
                            // Skip this transaction if "value" is missing
                            return None;
                        }

                        Some(Value::Object(new_transaction))
                    })
                    .collect();

                filtered_activity.insert(key.clone(), Value::Array(transformed_transactions));
            }

            "title" => {
                if let Some(narrative) = value.get("narrative") {
                    filtered_activity.insert(key.clone(), narrative.clone());
                }
            }
            "reporting-org" => {
                let mut new_reporting_org = Map::new();
                if let Some(ref_val) = value.get("ref") {
                    new_reporting_org.insert("id".to_string(), ref_val.clone());
                }
                if let Some(type_val) = value.get("type") {
                    new_reporting_org.insert("type".to_string(), type_val.clone());
                }
                if let Some(narrative) = value.get("narrative") {
                    new_reporting_org.insert("name".to_string(), narrative.clone());
                }
                filtered_activity.insert(key.clone(), Value::Object(new_reporting_org));
            }
            _ => {
                filtered_activity.insert(key.clone(), value.clone());
            }
        }
    }

    Value::Object(filtered_activity)
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
