# IATI XML to JSON Converter

Easily convert IATI XML data to a structured JSON format.

This is a Rust tool that can be used directly in Python, thanks to the pyo3 library. The package provides a function ```convert``` that, based on a given path, either processes a directory of XML files or a single XML file, converting them to a unified JSON format. If no path is provided, the package downloads a zip of current IATI data, processes it, and then performs the conversion.

## Quick Start

1. Clone the repository:
    ```bash
    gh repo clone eastcoasting/iati_json_parser
    cd iati_json_parser 
    ```

2. Set up the environment (skip the `unset` command if not required):
    ```bash
    unset CONDA_PREFIX  # Optional
    maturin develop
    pip install iati_json_parser
    ```

## How It Works

1. **Input Options**: Provide an IATI XML file or a set of folders. If you don't have these, the tool will automatically download the most recent version from the [daily snapshot of IATI Registry](https://iati-data-dump.codeforiati.org/).
   
2. **Data Processing**: 
   - Parse and convert all activity files to JSON format.
   - Ensure improved error handling for any malformed XML.

3. **Output**: The output is a well-structured JSON, an array of objects with the following signature:

```
{
        "default-currency": String,
        "last-updated-datetime": String,
        "iati-identifier": String,
        "reporting-org": {
            "id": String,
            "type": Number,
            "name": [
                {
                    "xml:lang": String,
                    "#text": String
                },
                ...
            ]
        },
        "title": {
            "xml:lang": String,
            "#text": String
        },
        "description": {
            "narrative": {
                "xml:lang": String,
                "#text": String
            }
        },
        "participating-org": [
            {
                "activity-id": String,
                "crs-channel-code": Number,
                "ref": String,
                "role": Number,
                "type": Number,
                "narrative": {
                    "xml:lang": String,
                    "#text": String
                }
            },
            ...
        ],
        "activity-status": {
            "code": Number
        },
        "recipient-country": {
            "code": String,
            "percentage": Number
        },
        "recipient-region": {
            "code": Number,
            "vocabulary": Number
        },
        "transaction": [
            {
                "transaction-type": Number,
                "transaction-date": Date,
                "transaction-value": Number
            },
            ...
        ]
}
```


---

### Credits:
This project is inspired by David Raznick's work on [iati2json](https://github.com/kindly/iati2json).

