use serde_json::{self, Value};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::Command;

#[test]
fn find_iati_id_gb_gov_1_300555_110() {
    let json_file_path = "iati_repository_2025-07-28.json";

    // Check if the file exists
    if !Path::new(json_file_path).exists() {
        panic!("JSON file not found: {}", json_file_path);
    }

    println!("Scanning JSON file for IATI ID: GB-GOV-1-300555-110");
    println!("Using streaming approach for large file...");

    // Open the file
    let file = File::open(json_file_path).expect("Failed to open JSON file");
    let reader = BufReader::new(file);

    let target_id = "GB-GOV-1-300555-110";
    let mut found_item: Option<String> = None;
    let mut item_count = 0;
    let mut in_array = false;
    let mut brace_count = 0;
    let mut current_item = String::new();
    let mut in_item = false;

    for line in reader.lines() {
        let line = line.expect("Failed to read line");

        for char in line.chars() {
            if !in_array && char == '[' {
                in_array = true;
                continue;
            }

            if in_array {
                if char == '{' {
                    if brace_count == 0 {
                        in_item = true;
                        current_item.clear();
                    }
                    brace_count += 1;
                }

                if in_item {
                    current_item.push(char);
                }

                if char == '}' {
                    brace_count -= 1;
                    if brace_count == 0 {
                        in_item = false;
                        item_count += 1;

                        // Try to parse the current item
                        if let Ok(item) = serde_json::from_str::<Value>(&current_item) {
                            if let Some(iati_id) = item.get("iati-identifier") {
                                if let Some(id_str) = iati_id.as_str() {
                                    if id_str == target_id {
                                        found_item = Some(current_item.clone());
                                        break;
                                    }
                                }
                            }
                        }

                        // Print progress every 10000 items
                        if item_count % 10000 == 0 {
                            println!("Processed {} items...", item_count);
                        }
                    }
                }
            }
        }

        if found_item.is_some() {
            break;
        }
    }

    match found_item {
        Some(item_json) => {
            println!("\n✅ Found IATI ID: {}", target_id);
            println!("Full struct:");

            // Parse and pretty print the found item
            if let Ok(item) = serde_json::from_str::<Value>(&item_json) {
                println!("{}", serde_json::to_string_pretty(&item).unwrap());
            } else {
                println!("{}", item_json);
            }
        }
        None => {
            println!("\n❌ IATI ID '{}' not found in the JSON file", target_id);
            println!("Processed {} total items", item_count);
        }
    }
}

#[test]
fn find_iati_id_with_simple_search() {
    let json_file_path = "iati_repository_2025-07-28.json";

    // Check if the file exists
    if !Path::new(json_file_path).exists() {
        panic!("JSON file not found: {}", json_file_path);
    }

    println!("Scanning JSON file for IATI ID: GB-GOV-1-300555-110");
    println!("Using simple text search approach...");

    // Open the file
    let file = File::open(json_file_path).expect("Failed to open JSON file");
    let reader = BufReader::new(file);

    let target_id = "GB-GOV-1-300555-110";
    let mut found_item: Option<String> = None;
    let mut item_count = 0;
    let mut in_array = false;
    let mut brace_count = 0;
    let mut current_item = String::new();
    let mut in_item = false;

    for line in reader.lines() {
        let line = line.expect("Failed to read line");

        for char in line.chars() {
            if !in_array && char == '[' {
                in_array = true;
                continue;
            }

            if in_array {
                if char == '{' {
                    if brace_count == 0 {
                        in_item = true;
                        current_item.clear();
                    }
                    brace_count += 1;
                }

                if in_item {
                    current_item.push(char);
                }

                if char == '}' {
                    brace_count -= 1;
                    if brace_count == 0 {
                        in_item = false;
                        item_count += 1;

                        // Check if this item contains our target ID
                        if current_item.contains(&format!("\"iati-identifier\": \"{}\"", target_id))
                        {
                            found_item = Some(current_item.clone());
                            break;
                        }

                        // Print progress every 10000 items
                        if item_count % 10000 == 0 {
                            println!("Processed {} items...", item_count);
                        }
                    }
                }
            }
        }

        if found_item.is_some() {
            break;
        }
    }

    match found_item {
        Some(item_json) => {
            println!("\n✅ Found IATI ID: {}", target_id);
            println!("Full struct:");

            // Parse and pretty print the found item
            if let Ok(item) = serde_json::from_str::<Value>(&item_json) {
                println!("{}", serde_json::to_string_pretty(&item).unwrap());
            } else {
                println!("{}", item_json);
            }
        }
        None => {
            println!("\n❌ IATI ID '{}' not found in the JSON file", target_id);
            println!("Processed {} total items", item_count);
        }
    }
}

#[test]
fn find_iati_id_with_sed_approach() {
    let json_file_path = "iati_repository_2025-07-28.json";

    // Check if the file exists
    if !Path::new(json_file_path).exists() {
        panic!("JSON file not found: {}", json_file_path);
    }

    println!("Finding IATI ID: GB-GOV-1-300555-110 using sed approach...");

    let target_id = "GB-GOV-1-300555-110";

    // First, find the line number where the ID appears
    let grep_output = Command::new("grep")
        .arg("-n")
        .arg(&format!("\"iati-identifier\": \"{}\"", target_id))
        .arg(json_file_path)
        .output()
        .expect("Failed to execute grep command");

    if grep_output.status.success() {
        let grep_result = String::from_utf8_lossy(&grep_output.stdout);
        let line_info = grep_result.trim();
        println!("Found ID at: {}", line_info);

        // Extract line number
        if let Some(line_num_str) = line_info.split(':').next() {
            if let Ok(line_num) = line_num_str.parse::<usize>() {
                println!("Line number: {}", line_num);

                // Use sed to extract lines around the target line (50 lines before and after)
                let start_line = if line_num > 50 { line_num - 50 } else { 1 };
                let end_line = line_num + 50;

                let sed_output = Command::new("sed")
                    .arg("-n")
                    .arg(&format!("{},{}p", start_line, end_line))
                    .arg(json_file_path)
                    .output()
                    .expect("Failed to execute sed command");

                if sed_output.status.success() {
                    let extracted_lines = String::from_utf8_lossy(&sed_output.stdout);
                    println!(
                        "Extracted {} lines around target line",
                        extracted_lines.lines().count()
                    );

                    // Now find the complete JSON object in the extracted lines
                    let mut brace_count = 0;
                    let mut in_item = false;
                    let mut current_item = String::new();
                    let mut found_item: Option<String> = None;

                    for line in extracted_lines.lines() {
                        for char in line.chars() {
                            if char == '{' {
                                if brace_count == 0 {
                                    in_item = true;
                                    current_item.clear();
                                }
                                brace_count += 1;
                            }

                            if in_item {
                                current_item.push(char);
                            }

                            if char == '}' {
                                brace_count -= 1;
                                if brace_count == 0 {
                                    in_item = false;

                                    // Check if this item contains our target ID
                                    if current_item.contains(&format!(
                                        "\"iati-identifier\": \"{}\"",
                                        target_id
                                    )) {
                                        found_item = Some(current_item.clone());
                                        break;
                                    }
                                }
                            }
                        }

                        if found_item.is_some() {
                            break;
                        }
                    }

                    match found_item {
                        Some(item_json) => {
                            println!("\n✅ Found IATI ID: {}", target_id);
                            println!("Full struct:");

                            // Parse and pretty print the found item
                            if let Ok(item) = serde_json::from_str::<Value>(&item_json) {
                                println!("{}", serde_json::to_string_pretty(&item).unwrap());
                            } else {
                                println!("{}", item_json);
                            }
                        }
                        None => {
                            println!(
                                "\n❌ Could not extract complete JSON object for IATI ID: {}",
                                target_id
                            );
                            println!("Extracted lines content:");
                            println!("{}", extracted_lines);
                        }
                    }
                } else {
                    println!("Failed to extract lines with sed");
                }
            }
        }
    } else {
        println!("\n❌ IATI ID '{}' not found in the JSON file", target_id);
    }
}

#[test]
fn find_iati_id_complete_extraction() {
    let json_file_path = "iati_repository_2025-07-28.json";

    // Check if the file exists
    if !Path::new(json_file_path).exists() {
        panic!("JSON file not found: {}", json_file_path);
    }

    println!("Finding IATI ID: GB-GOV-1-300555-110 with complete extraction...");

    let target_id = "GB-GOV-1-300555-110";

    // First, find the line number where the ID appears
    let grep_output = Command::new("grep")
        .arg("-n")
        .arg(&format!("\"iati-identifier\": \"{}\"", target_id))
        .arg(json_file_path)
        .output()
        .expect("Failed to execute grep command");

    if grep_output.status.success() {
        let grep_result = String::from_utf8_lossy(&grep_output.stdout);
        let line_info = grep_result.trim();
        println!("Found ID at: {}", line_info);

        // Extract line number
        if let Some(line_num_str) = line_info.split(':').next() {
            if let Ok(line_num) = line_num_str.parse::<usize>() {
                println!("Line number: {}", line_num);

                // Use sed to extract a larger range around the target line (500 lines before and after)
                let start_line = if line_num > 500 { line_num - 500 } else { 1 };
                let end_line = line_num + 500;

                let sed_output = Command::new("sed")
                    .arg("-n")
                    .arg(&format!("{},{}p", start_line, end_line))
                    .arg(json_file_path)
                    .output()
                    .expect("Failed to execute sed command");

                if sed_output.status.success() {
                    let extracted_lines = String::from_utf8_lossy(&sed_output.stdout);
                    println!(
                        "Extracted {} lines around target line",
                        extracted_lines.lines().count()
                    );

                    // Now find the complete JSON object in the extracted lines
                    let mut brace_count = 0;
                    let mut in_item = false;
                    let mut current_item = String::new();
                    let mut found_item: Option<String> = None;

                    for line in extracted_lines.lines() {
                        for char in line.chars() {
                            if char == '{' {
                                if brace_count == 0 {
                                    in_item = true;
                                    current_item.clear();
                                }
                                brace_count += 1;
                            }

                            if in_item {
                                current_item.push(char);
                            }

                            if char == '}' {
                                brace_count -= 1;
                                if brace_count == 0 {
                                    in_item = false;

                                    // Check if this item contains our target ID
                                    if current_item.contains(&format!(
                                        "\"iati-identifier\": \"{}\"",
                                        target_id
                                    )) {
                                        found_item = Some(current_item.clone());
                                        break;
                                    }
                                }
                            }
                        }

                        if found_item.is_some() {
                            break;
                        }
                    }

                    match found_item {
                        Some(item_json) => {
                            println!("\n✅ Found IATI ID: {}", target_id);
                            println!("Full struct:");

                            // Parse and pretty print the found item
                            if let Ok(item) = serde_json::from_str::<Value>(&item_json) {
                                println!("{}", serde_json::to_string_pretty(&item).unwrap());
                            } else {
                                println!("{}", item_json);
                            }
                        }
                        None => {
                            println!(
                                "\n❌ Could not extract complete JSON object for IATI ID: {}",
                                target_id
                            );
                            println!("Trying alternative approach...");

                            // Try to find the object by looking for the pattern in the extracted lines
                            let lines: Vec<&str> = extracted_lines.lines().collect();
                            let mut start_idx = None;
                            let mut end_idx = None;

                            for (i, line) in lines.iter().enumerate() {
                                if line.contains(&format!("\"iati-identifier\": \"{}\"", target_id))
                                {
                                    // Look backwards for the start of the object
                                    for j in (0..i).rev() {
                                        if lines[j].trim().starts_with('{') {
                                            start_idx = Some(j);
                                            break;
                                        }
                                    }

                                    // Look forwards for the end of the object
                                    for j in i..lines.len() {
                                        if lines[j].trim().ends_with('}') {
                                            end_idx = Some(j);
                                            break;
                                        }
                                    }
                                    break;
                                }
                            }

                            if let (Some(start), Some(end)) = (start_idx, end_idx) {
                                let object_lines = &lines[start..=end];
                                let object_text = object_lines.join("\n");

                                println!(
                                    "Extracted object from lines {} to {}",
                                    start + 1,
                                    end + 1
                                );
                                println!("Full struct:");

                                if let Ok(item) = serde_json::from_str::<Value>(&object_text) {
                                    println!("{}", serde_json::to_string_pretty(&item).unwrap());
                                } else {
                                    println!("{}", object_text);
                                }
                            } else {
                                println!("Could not determine object boundaries");
                            }
                        }
                    }
                } else {
                    println!("Failed to extract lines with sed");
                }
            }
        }
    } else {
        println!("\n❌ IATI ID '{}' not found in the JSON file", target_id);
    }
}

#[test]
fn find_iati_id_final_extraction() {
    let json_file_path = "iati_repository_2025-07-28.json";

    // Check if the file exists
    if !Path::new(json_file_path).exists() {
        panic!("JSON file not found: {}", json_file_path);
    }

    println!("Finding IATI ID with final extraction...");

    let target_id = "44000-P160033";

    // First, find the line number where the ID appears
    let grep_output = Command::new("grep")
        .arg("-n")
        .arg(&format!("\"iati-identifier\": \"{}\"", target_id))
        .arg(json_file_path)
        .output()
        .expect("Failed to execute grep command");

    if grep_output.status.success() {
        let grep_result = String::from_utf8_lossy(&grep_output.stdout);
        let line_info = grep_result.trim();
        println!("Found ID at: {}", line_info);

        // Extract line number
        if let Some(line_num_str) = line_info.split(':').next() {
            if let Ok(line_num) = line_num_str.parse::<usize>() {
                println!("Line number: {}", line_num);

                // Use sed to extract a much larger range around the target line (2000 lines before and after)
                let start_line = if line_num > 2000 { line_num - 2000 } else { 1 };
                let end_line = line_num + 2000;

                let sed_output = Command::new("sed")
                    .arg("-n")
                    .arg(&format!("{},{}p", start_line, end_line))
                    .arg(json_file_path)
                    .output()
                    .expect("Failed to execute sed command");

                if sed_output.status.success() {
                    let extracted_lines = String::from_utf8_lossy(&sed_output.stdout);
                    println!(
                        "Extracted {} lines around target line",
                        extracted_lines.lines().count()
                    );

                    // Try to find the object by looking for the pattern in the extracted lines
                    let lines: Vec<&str> = extracted_lines.lines().collect();
                    let mut start_idx = None;
                    let mut end_idx = None;

                    for (i, line) in lines.iter().enumerate() {
                        if line.contains(&format!("\"iati-identifier\": \"{}\"", target_id)) {
                            // Look backwards for the start of the object
                            for j in (0..i).rev() {
                                if lines[j].trim().starts_with('{') {
                                    start_idx = Some(j);
                                    break;
                                }
                            }

                            // Look forwards for the end of the object
                            for j in i..lines.len() {
                                if lines[j].trim().ends_with('}') {
                                    end_idx = Some(j);
                                    break;
                                }
                            }
                            break;
                        }
                    }

                    if let (Some(start), Some(end)) = (start_idx, end_idx) {
                        let object_lines = &lines[start..=end];
                        let object_text = object_lines.join("\n");

                        println!("Extracted object from lines {} to {}", start + 1, end + 1);
                        println!("Full struct:");

                        if let Ok(item) = serde_json::from_str::<Value>(&object_text) {
                            println!("{}", serde_json::to_string_pretty(&item).unwrap());
                        } else {
                            println!("{}", object_text);
                        }
                    } else {
                        println!("Could not determine object boundaries");
                    }
                } else {
                    println!("Failed to extract lines with sed");
                }
            }
        }
    } else {
        println!("\n❌ IATI ID '{}' not found in the JSON file", target_id);
    }
}

#[test]
fn find_iati_id_with_jq() {
    let json_file_path = "iati_repository_2025-07-28.json";

    // Check if the file exists
    if !Path::new(json_file_path).exists() {
        panic!("JSON file not found: {}", json_file_path);
    }

    println!("Finding IATI ID using jq...");

    let target_id = "44000-P160033";

    // Use jq to find and extract the complete JSON object
    let jq_output = Command::new("jq")
        .arg(&format!(
            ".[] | select(.[\"iati-identifier\"] == \"{}\")",
            target_id
        ))
        .arg(json_file_path)
        .output()
        .expect("Failed to execute jq command");

    if jq_output.status.success() {
        let result = String::from_utf8_lossy(&jq_output.stdout);
        if !result.trim().is_empty() {
            println!("\n✅ Found IATI ID: {}", target_id);
            println!("Full struct:");
            println!("{}", result);
        } else {
            println!("\n❌ IATI ID '{}' not found in the JSON file", target_id);
        }
    } else {
        println!("\n❌ Failed to execute jq command or IATI ID not found");
        println!("jq error: {}", String::from_utf8_lossy(&jq_output.stderr));
    }
}
