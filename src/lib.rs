use std::fs::{self, OpenOptions, File};
use std::io::Read;
use std::path::Path;
use quickxml_to_serde::{xml_string_to_json, Config, JsonType, JsonArray};
use pyo3::prelude::*;
use regex::Regex;
use eyre::Result;
use serde_json::Value;

use chardet::{detect, charset2encoding};
use encoding::DecoderTrap;
use encoding::label::encoding_from_whatwg_label;


const ARRAY_PATHS: &'static [&'static str] = 
   &["/iati-activities/iati-activity",
    "/iati-activities/iati-activity/reporting-org/narrative",
    "/iati-activities/iati-activity/title/narrative",
    "/iati-activities/iati-activity/description",
    "/iati-activities/iati-activity/description/narrative",
    "/iati-activities/iati-activity/participating-org",
    "/iati-activities/iati-activity/participating-org/narrative",
    "/iati-activities/iati-activity/other-identifier",
    "/iati-activities/iati-activity/other-identifier/owner-org/narrative",
    "/iati-activities/iati-activity/activity-date",
    "/iati-activities/iati-activity/activity-date/narrative",
    "/iati-activities/iati-activity/contact-info",
    "/iati-activities/iati-activity/contact-info/organisation/narrative",
    "/iati-activities/iati-activity/contact-info/department/narrative",
    "/iati-activities/iati-activity/contact-info/person-name/narrative",
    "/iati-activities/iati-activity/contact-info/job-title/narrative",
    "/iati-activities/iati-activity/contact-info/telephone",
    "/iati-activities/iati-activity/contact-info/email",
    "/iati-activities/iati-activity/contact-info/website",
    "/iati-activities/iati-activity/contact-info/mailing-address",
    "/iati-activities/iati-activity/contact-info/mailing-address/narrative",
    "/iati-activities/iati-activity/recipient-country",
    "/iati-activities/iati-activity/recipient-country/narrative",
    "/iati-activities/iati-activity/recipient-region",
    "/iati-activities/iati-activity/recipient-region/narrative",
    "/iati-activities/iati-activity/location",
    "/iati-activities/iati-activity/location/location-id",
    "/iati-activities/iati-activity/location/name/narrative",
    "/iati-activities/iati-activity/location/description/narrative",
    "/iati-activities/iati-activity/location/activity-description/narrative",
    "/iati-activities/iati-activity/location/administrative",
    "/iati-activities/iati-activity/sector",
    "/iati-activities/iati-activity/sector/narrative",
    "/iati-activities/iati-activity/tag",
    "/iati-activities/iati-activity/tag/narrative",
    "/iati-activities/iati-activity/country-budget-items/budget-item",
    "/iati-activities/iati-activity/country-budget-items/budget-item/description/narrative",
    "/iati-activities/iati-activity/humanitarian-scope",
    "/iati-activities/iati-activity/humanitarian-scope/narrative",
    "/iati-activities/iati-activity/policy-marker",
    "/iati-activities/iati-activity/policy-marker/narrative",
    "/iati-activities/iati-activity/default-aid-type",
    "/iati-activities/iati-activity/budget",
    "/iati-activities/iati-activity/planned-disbursement",
    "/iati-activities/iati-activity/planned-disbursement/provider-org/narrative",
    "/iati-activities/iati-activity/planned-disbursement/receiver-org/narrative",
    "/iati-activities/iati-activity/transaction",
    "/iati-activities/iati-activity/transaction/description/narrative",
    "/iati-activities/iati-activity/transaction/provider-org/narrative",
    "/iati-activities/iati-activity/transaction/receiver-org/narrative",
    "/iati-activities/iati-activity/transaction/sector",
    "/iati-activities/iati-activity/transaction/sector/narrative",
    "/iati-activities/iati-activity/transaction/recipient-country/narrative",
    "/iati-activities/iati-activity/transaction/recipient-region/narrative",
    "/iati-activities/iati-activity/transaction/aid-type",
    "/iati-activities/iati-activity/document-link",
    "/iati-activities/iati-activity/document-link/title/narrative",
    "/iati-activities/iati-activity/document-link/description/narrative",
    "/iati-activities/iati-activity/document-link/category",
    "/iati-activities/iati-activity/document-link/language",
    "/iati-activities/iati-activity/related-activity",
    "/iati-activities/iati-activity/legacy-data",
    "/iati-activities/iati-activity/conditions/condition",
    "/iati-activities/iati-activity/conditions/condition/narrative",
    "/iati-activities/iati-activity/result",
    "/iati-activities/iati-activity/result/title/narrative",
    "/iati-activities/iati-activity/result/description/narrative",
    "/iati-activities/iati-activity/result/document-link",
    "/iati-activities/iati-activity/result/document-link/title/narrative",
    "/iati-activities/iati-activity/result/document-link/description/narrative",
    "/iati-activities/iati-activity/result/document-link/category",
    "/iati-activities/iati-activity/result/document-link/language",
    "/iati-activities/iati-activity/result/reference",
    "/iati-activities/iati-activity/result/indicator",
    "/iati-activities/iati-activity/result/indicator/title/narrative",
    "/iati-activities/iati-activity/result/indicator/description/narrative",
    "/iati-activities/iati-activity/result/indicator/document-link",
    "/iati-activities/iati-activity/result/indicator/document-link/title/narrative",
    "/iati-activities/iati-activity/result/indicator/document-link/description/narrative",
    "/iati-activities/iati-activity/result/indicator/document-link/category",
    "/iati-activities/iati-activity/result/indicator/document-link/language",
    "/iati-activities/iati-activity/result/indicator/reference",
    "/iati-activities/iati-activity/result/indicator/baseline",
    "/iati-activities/iati-activity/result/indicator/baseline/location",
    "/iati-activities/iati-activity/result/indicator/baseline/dimension",
    "/iati-activities/iati-activity/result/indicator/baseline/document-link",
    "/iati-activities/iati-activity/result/indicator/baseline/document-link/title/narrative",
    "/iati-activities/iati-activity/result/indicator/baseline/document-link/description/narrative",
    "/iati-activities/iati-activity/result/indicator/baseline/document-link/category",
    "/iati-activities/iati-activity/result/indicator/baseline/document-link/language",
    "/iati-activities/iati-activity/result/indicator/baseline/comment/narrative",
    "/iati-activities/iati-activity/result/indicator/period",
    "/iati-activities/iati-activity/result/indicator/period/target",
    "/iati-activities/iati-activity/result/indicator/period/target/location",
    "/iati-activities/iati-activity/result/indicator/period/target/dimension",
    "/iati-activities/iati-activity/result/indicator/period/target/comment/narrative",
    "/iati-activities/iati-activity/result/indicator/period/target/document-link",
    "/iati-activities/iati-activity/result/indicator/period/target/document-link/title/narrative",
    "/iati-activities/iati-activity/result/indicator/period/target/document-link/description/narrative",
    "/iati-activities/iati-activity/result/indicator/period/target/document-link/category",
    "/iati-activities/iati-activity/result/indicator/period/target/document-link/language",
    "/iati-activities/iati-activity/result/indicator/period/actual",
    "/iati-activities/iati-activity/result/indicator/period/actual/location",
    "/iati-activities/iati-activity/result/indicator/period/actual/dimension",
    "/iati-activities/iati-activity/result/indicator/period/actual/comment/narrative",
    "/iati-activities/iati-activity/result/indicator/period/actual/document-link",
    "/iati-activities/iati-activity/result/indicator/period/actual/document-link/title/narrative",
    "/iati-activities/iati-activity/result/indicator/period/actual/document-link/description/narrative",
    "/iati-activities/iati-activity/result/indicator/period/actual/document-link/category",
    "/iati-activities/iati-activity/result/indicator/period/actual/document-link/language",
    "/iati-activities/iati-activity/crs-add/other-flags",
    "/iati-activities/iati-activity/fss/forecast"];


fn try_convert_xml_to_json(xml: &str, config: &Config) -> Result<String, eyre::Report> {
    let json = xml_string_to_json((&xml).to_string(), config)?;
    Ok(serde_json::to_string(&json)?)
}
    
    
fn preprocess_xml(input: &str) -> String {
    let mut processed_xml = input.to_string();
    println!("Preprocessing file");
    
    // Remove URLs encoded see gef-mx example
    let re = Regex::new(r"<https?://[^>]*>").unwrap();
    processed_xml = re.replace_all(&processed_xml, "").to_string();

    return processed_xml
}
    


fn read_and_decode_xml(path: &Path) -> Result<String> {
    let mut fh = OpenOptions::new().read(true).open(path)?;
    let mut reader: Vec<u8> = Vec::new();
    fh.read_to_end(&mut reader)?;

    let result = detect(&reader);
    let coder = encoding_from_whatwg_label(charset2encoding(&result.0));
    if let Some(coder) = coder {
        coder.decode(&reader, DecoderTrap::Ignore).map_err(|e| eyre::eyre!(e))
    } else {
        let mut xml_contents = String::new();
        File::open(path)?.read_to_string(&mut xml_contents)?;
        Ok(xml_contents)
    }
}

fn convert_xml_to_json(xml_content: &String, arrays: &Option<Vec<String>>) -> Result<Value> {
    let mut config = Config::new_with_defaults();

    if let Some(arrays) = arrays {
        for path in arrays {
            config.json_type_overrides.insert(path.to_string(), JsonArray::Always(JsonType::Infer));
        }
    } else {
        for path in ARRAY_PATHS {
            config.json_type_overrides.insert(path.to_string(), JsonArray::Always(JsonType::Infer));
        }
    }
    config.xml_attr_prefix = "".into();

    match try_convert_xml_to_json(&xml_content, &config) {
        Ok(json_str) => Ok(serde_json::from_str(&json_str)?),
        Err(_) => {
            let preprocessed_xml = preprocess_xml(&xml_content);
            Ok(xml_string_to_json(preprocessed_xml, &config)?)
        }
    }
}

fn walk_and_convert(dir: &Path, arrays: &Option<Vec<String>>, unified_json: &mut Vec<Value>) -> eyre::Result<()> {
    let entries = fs::read_dir(dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_and_convert(&path, &arrays, unified_json)?;
        } else if path.is_file() && path.extension().map_or(false, |ext| ext == "xml") && !path.to_string_lossy().ends_with("-org.xml") {
            let xml_content = read_and_decode_xml(&path)?;
            let json = convert_xml_to_json(&xml_content, &arrays)?;
            unified_json.push(json);
        }
    }
    Ok(())
}

#[pyfunction]
pub fn convert(
    path: String, 
    pretty: Option<bool>, 
    arrays: Option<Vec<String>>
) -> Result<Option<String>> {
    let p = Path::new(&path);

    let mut unified_json: Vec<Value> = Vec::new();

    if p.is_dir() {

        walk_and_convert(&p, &arrays, &mut unified_json)?;
        
        let result = if pretty.unwrap_or(false) {
            serde_json::to_string_pretty(&unified_json)?
        } else {
            serde_json::to_string(&unified_json)?
        };


        Ok(Some(result))
    } else if p.is_file() && p.extension().map_or(true, |ext| ext != "xml") && !path.ends_with("-org.xml") {
        let xml_content = read_and_decode_xml(&p)?;
        let json = convert_xml_to_json(&xml_content, &arrays)?;

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
