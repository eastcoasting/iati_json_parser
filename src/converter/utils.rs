use reqwest;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;
use zip::ZipArchive;

use eyre::Result;
use quickxml_to_serde::xml_string_to_json;
use quickxml_to_serde::Config;
use regex::Regex;
use serde_json::Value;
use std::collections::HashSet;

use std::path::Path;

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
    println!("Preprocessing file");

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
                        let transaction_type =
                            transaction.get("transaction-type")?.get("code")?.clone();
                        let transaction_date = transaction
                            .get("transaction-date")?
                            .get("iso-date")?
                            .clone();
                        let transaction_value = transaction.get("value")?.get("#text")?.clone();

                        new_transaction.insert("transaction-type".to_string(), transaction_type);
                        new_transaction.insert("transaction-date".to_string(), transaction_date);
                        new_transaction.insert("transaction-value".to_string(), transaction_value);

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

    // Return the path to the `data` folder
    Ok(root_path.join("data").to_string_lossy().into_owned())
}
