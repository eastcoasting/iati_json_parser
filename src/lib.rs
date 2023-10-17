use pyo3::prelude::*;

use std::path::Path;

use indicatif::ProgressBar;

use converter::{
    convert_xml_to_json, read_and_decode_xml,
    utils::{count_xml_files, download_zip, process_root},
    walk_and_convert,
};

use pyo3::{pymodule, types::PyModule, wrap_pyfunction, PyResult, Python};

pub mod converter;

#[pyfunction]
pub fn convert(path: Option<String>, pretty: Option<bool>) -> eyre::Result<Option<String>> {
    let extraction_path: String;

    if let Some(provided_path) = path {
        extraction_path = provided_path;
    } else {
        println!(
            "Beginning download of source data. This is a large file, timeout set to 15 minutes."
        );
        // Download the zip file if no path is provided
        let zip_path = download_zip(
            "https://gitlab.com/codeforIATI/iati-data/-/archive/main/iati-data-main.zip",
        )?;

        extraction_path = process_root(&zip_path)?;
    }

    let p = Path::new(&extraction_path);

    let total_files = count_xml_files(&p)?;
    let mut current_count = 0;
    let pb = ProgressBar::new(total_files as u64);

    if p.is_dir() {
        let unified_json = walk_and_convert(&p, &mut current_count, &pb)?;

        let result = if pretty.unwrap_or(false) {
            serde_json::to_string_pretty(&unified_json)?
        } else {
            serde_json::to_string(&unified_json)?
        };

        Ok(Some(result))
    } else if p.is_file()
        && p.extension().map_or(true, |ext| ext != "xml")
        && !extraction_path.ends_with("-org.xml")
    {
        let xml_content = read_and_decode_xml(&p)?;
        let json = convert_xml_to_json(&xml_content)?;

        let result = if pretty.unwrap_or(false) {
            serde_json::to_string_pretty(&json)?
        } else {
            serde_json::to_string(&json)?
        };

        Ok(Some(result))
    } else {
        Err(eyre::eyre!("Invalid path provided. Ensure it's either an XML file or a directory containing XML files."))
    }
}

#[pymodule]
fn iati_json_parser(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(convert, m)?)?;
    Ok(())
}
