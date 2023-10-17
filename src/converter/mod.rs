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

fn read_xml(path: &Path) -> eyre::Result<String> {
    let content = fs::read_to_string(path)
        .map_err(|e| eyre::eyre!("Error reading XML file {:?}: {:?}", &path, e))?;

    // Check for XML declaration or absence of HTML declaration
    if !content.starts_with("<?xml") && content.contains("<!DOCTYPE html>") {
        return Err(eyre::eyre!(
            "The file {:?} does not seem to be a valid XML.",
            path
        ));
    }

    Ok(content).or_else(|_err: std::io::Error| {
        let bytes = fs::read(path).map_err(eyre::Report::from)?;
        let decoded = String::from_utf16(
            &bytes
                .as_slice()
                .chunks_exact(2)
                .map(|chunk| u16::from_ne_bytes([chunk[0], chunk[1]]))
                .collect::<Vec<u16>>(),
        )
        .map_err(eyre::Report::from)?;
        Ok(decoded)
    })
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
                            println!("Error reading XML file {:?}: {}", &path, e);
                            return Err(eyre::eyre!("Error reading XML file {:?}: {}", &path, e));
                        }
                    };

                    let json = match convert_xml_to_json(&xml_content) {
                        Ok(j) => j,
                        Err(e) => {
                            println!("Error converting XML file {:?} to JSON: {}", &path, e);
                            return Err(eyre::eyre!(
                                "Error converting XML file {:?} to JSON: {}",
                                &path,
                                e
                            ));
                        }
                    };

                    let mut unified = Vec::new();

                    if let Some(activities) = json.get("iati-activities") {
                        if let Some(activity_array) =
                            activities.get("iati-activity").and_then(Value::as_array)
                        {
                            for activity in activity_array {
                                let filtered = filter_activity(activity);
                                unified.push(filtered);
                            }
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
