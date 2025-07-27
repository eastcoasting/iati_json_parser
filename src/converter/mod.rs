use eyre::Result;

use quickxml_to_serde::{xml_string_to_json, Config};
use serde_json::Value;
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

use rayon::prelude::*;

use indicatif::ProgressBar;

use crate::converter::utils::filter_activity;

use self::utils::{preprocess_xml, try_convert_xml_to_json};

pub mod utils;


use eyre::Report;

fn read_xml(path: &Path) -> Result<String, Report> {
    let attempt = fs::read_to_string(path);
    match attempt {
        Ok(content) => {
            // Check for XML declaration or absence of HTML declaration
            if !content.starts_with("<?xml") && content.contains("<!DOCTYPE html>") {
                Err(eyre::eyre!(
                    "The file {:?} does not seem to be a valid XML.",
                    path
                ))
            } else {
                Ok(content)
            }
        }
        Err(_e) => {
            // Read raw bytes
            let bytes = fs::read(path)?;

            // Check the first two bytes to guess the encoding
            let decoded = if bytes.starts_with(&[0xFF, 0xFE]) {
                // Detected UTF-16LE
                let utf16_values: Vec<u16> = bytes
                    .chunks_exact(2)
                    .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                    .collect();
                String::from_utf16(&utf16_values)
            } else if bytes.starts_with(&[0xFE, 0xFF]) {
                // Detected UTF-16BE
                let utf16_values: Vec<u16> = bytes
                    .chunks_exact(2)
                    .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
                    .collect();
                String::from_utf16(&utf16_values)
            } else {
                return Err(eyre::eyre!("Unable to determine encoding for {:?}", path));
            };

            match decoded {
                Ok(content) => {
                    if !content.starts_with("<?xml") && content.contains("<!DOCTYPE html>") {
                        Err(eyre::eyre!(
                            "The file {:?} decoded does not seem to be a valid XML.",
                            path
                        ))
                    } else {
                        Ok(content)
                    }
                }
                Err(e) => Err(eyre::eyre!("UTF-16 decoding error: {:?}", e)),
            }
        }
    }
}


pub fn read_and_decode_xml(path: &Path) -> Result<String> {
    let mut xml_contents = String::new();
    File::open(path)?.read_to_string(&mut xml_contents)?;

    if xml_contents.trim().is_empty() {
        return Err(eyre::eyre!("The file {:?} is empty.", path));
    }

    Ok(xml_contents)
}

pub fn convert_xml_to_json(xml_content: &String) -> Result<Value> {
    let mut config = Config::new_with_defaults();

    config.xml_attr_prefix = "".into();

    match try_convert_xml_to_json(&xml_content, &config) {
        Ok(json_str) => Ok(serde_json::from_str(&json_str)?),
        Err(e) => {
            let preprocessed_xml = preprocess_xml(&xml_content);

            xml_string_to_json(preprocessed_xml, &config).map_err(|_| {
                eyre::eyre!(
                    "Failed conversion after preprocessing. Original error: {}",
                    e
                )
            })
        }
    }
}

// Parallel version of walk_and_convert
pub fn walk_and_convert(
    dir: &Path,
    current_count: &mut usize,
    pb: &ProgressBar,
) -> eyre::Result<Vec<Value>> {
    let entries: Vec<_> = fs::read_dir(dir)?
        .map(|entry_res| {
            entry_res
                .map_err(eyre::Report::from) // Convert std::io::Error to eyre::Report
                .map(|entry| entry.path())
        })
        .collect::<eyre::Result<Vec<_>>>()?; // Now this will work since we've converted each entry

    let thread_results: Vec<_> = entries
        .par_iter()
        .map_init(
            || (0_usize, Vec::<Value>::new()),
            |(local_count, _local_unified), path| {
                let file_name = path
                    .file_name()
                    .unwrap_or_else(|| std::ffi::OsStr::new("Unknown"))
                    .to_string_lossy();

                if path.is_dir() {
                    walk_and_convert(&path, &mut 0, &pb)
                } else if path.is_file()
                    && path.extension().map_or(false, |ext| ext == "xml")
                    && !path.to_string_lossy().ends_with("-org.xml")
                {
                    let xml_content = match read_xml(&path) {
                        Ok(content) => content,
                        Err(e) => {
                            return Err(eyre::eyre!("Error reading XML file {:?}: {}", &path, e));
                        }
                    };

                    let json = match convert_xml_to_json(&xml_content) {
                        Ok(j) => j,
                        Err(e) => {
                            return Err(eyre::eyre!(
                                "Error converting XML file {:?} to JSON: {}",
                                &path,
                                e
                            ));
                        }
                    };

                    let mut unified = Vec::new();

                    if let Some(activities) = json.get("iati-activities") {
                    
                        if let Some(activity_array) = activities.get("iati-activity").and_then(Value::as_array) {

                            println!("Number of activities found: {}", activity_array.len());

                            for activity in activity_array {
                                let filtered = filter_activity(activity);
                                unified.push(filtered);
                            }
                        } else if activities.get("iati-activity").is_some() {
                            // Handle the single "iati-activity" that's not in array format
                            let filtered = filter_activity(&activities.get("iati-activity").unwrap()); // unwrap is safe here due to the previous check
                            unified.push(filtered);
                        } else {
                            println!("\n'iati-activity' key inside 'iati-activities' either doesn't exist or isn't an array.");
                        }
                    } else {
                        let filtered = filter_activity(&json);
                        unified.push(filtered);
                    }
                    

                    *local_count += 1;

                    println!(
                        "Processing file: '{}'; in folder: {:?}",
                        file_name,
                        path.parent().unwrap_or(&Path::new("Unknown"))
                    );

                    println!("Final output length: {}", unified.len());


                    pb.inc(1);
                    Ok(unified)
                } else {
                    Ok(Vec::new()) // Return an empty vector for unwanted paths
                }
            },
        )
        .collect::<Vec<eyre::Result<Vec<Value>>>>();



    // Aggregate the results and update the global counter
    let mut results: Vec<Value> = Vec::new();
    for local_unified_result in thread_results.into_iter() {
        match local_unified_result {
            Ok(local_unified) => {
                *current_count += local_unified.len();
                pb.inc(local_unified.len() as u64);
                results.extend(local_unified);
            }
            Err(e) => {
                println!("Error processing a file: {}", e);
            }
        }
    }

    Ok(results)
}

