# IATI XML to JSON Converter

Easily convert IATI XML data to a structured JSON format.

This is a Rust tool that can be used directly in Python, thanks to the pyo3 library. The package provides a function `convert` that, based on a given path, either processes a directory of XML files or a single XML file, converting them to a unified JSON format. If no path is provided, the package downloads a zip of current IATI data, processes it, and then performs the conversion.

## Quick Start

### Option 1: Direct Installation (Recommended)

1. **Prerequisites**: Make sure you have Rust and Python 3.8+ installed
   ```bash
   # Install Rust (if not already installed)
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   
   # Install Python 3.8+ (if not already installed)
   # Visit https://python.org/downloads/
   ```

2. **Clone and Install**:
   ```bash
   git clone https://github.com/eastcoasting/iati_json_parser.git
   cd iati_json_parser
   ./install.sh
   ```

### Option 2: Manual Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/eastcoasting/iati_json_parser.git
   cd iati_json_parser 
   ```

2. Set up the environment:
   ```bash
   pip install maturin
   maturin develop --release
   ```

### Option 3: From GitHub (Future - when published to PyPI)

```bash
pip install iati_json_parser
```

## Usage

### Basic Usage

```python
from iati_json_parser import convert

# Convert a single XML file
result = convert("path/to/your/file.xml")
print(result)

# Convert with pretty printing
result = convert("path/to/your/file.xml", pretty=True)
print(result)

# Convert a directory of XML files
result = convert("path/to/xml/directory")
print(result)

# Download and convert latest IATI data (no path provided)
result = convert()
print(result)
```

### Example Output

The output is a well-structured JSON array of objects with the following signature:

```json
[
  {
    "default-currency": "USD",
    "last-updated-datetime": "2024-01-15T10:30:00Z",
    "iati-identifier": "XM-DAC-12345-ABC123",
    "reporting-org": {
      "id": "XM-DAC-12345",
      "type": 10,
      "name": "Example Organization"
    },
    "title": {
      "xml:lang": "en",
      "#text": "Example Project Title"
    },
    "description": [
      {
        "narrative": {
          "xml:lang": "en",
          "#text": "Project description"
        }
      }
    ],
    "participating-org": [
      {
        "activity-id": "XM-DAC-12345-ABC123",
        "ref": "XM-DAC-12345",
        "role": 1,
        "type": 10,
        "narrative": {
          "xml:lang": "en",
          "#text": "Implementing Organization"
        }
      }
    ],
    "activity-status": {
      "code": 2
    },
    "recipient-country": {
      "code": "KE",
      "percentage": 100
    },
    "recipient-region": {
      "code": 202,
      "vocabulary": 1
    },
    "transaction": [
      {
        "transaction-type": 1,
        "transaction-date": "2024-01-15",
        "transaction-value": 1000000,
        "transaction-currency": "USD"
      }
    ]
  }
]
```

## Development

### Building for Development

```bash
# Install in development mode
maturin develop

# Run tests
python -m pytest
```

### Building for Distribution

```bash
# Build wheels
maturin build --release

# The wheels will be in target/wheels/
```

## How It Works

1. **Input Options**: Provide an IATI XML file or a set of folders. If you don't have these, the tool will automatically download the most recent version from the [daily snapshot of IATI Registry](https://iati-data-dump.codeforiati.org/).
   
2. **Data Processing**: 
   - Parse and convert all activity files to JSON format.
   - Ensure improved error handling for any malformed XML.
   - Process files in parallel for better performance.

3. **Output**: The output is a well-structured JSON array of activity objects.

## Performance

- **Parallel Processing**: Uses Rust's rayon library for parallel XML processing
- **Memory Efficient**: Processes files one at a time to manage memory usage
- **Progress Tracking**: Shows progress bars during conversion

## Troubleshooting

### Common Issues

1. **Rust not found**: Install Rust using `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

2. **Python version too old**: Ensure you have Python 3.8 or higher

3. **Build errors**: Make sure you have the latest Rust toolchain:
   ```bash
   rustup update
   ```

4. **Import errors**: After installation, restart your Python environment

### Getting Help

- Check the [GitHub Issues](https://github.com/eastcoasting/iati_json_parser/issues)
- Review the [CI logs](https://github.com/eastcoasting/iati_json_parser/actions) for build status

---

### Credits:
This project is inspired by David Raznick's work on [iati2json](https://github.com/kindly/iati2json).

