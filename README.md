# Convert IATI XML to JSON

##  Install

```
pip install iati_json_parser
```

---
Modifying David Raznick's work in [iati2json](https://github.com/kindly/iati2json) do the following:
- Download full IATI repository from [daily snapshot of IATI Registry](https://iati-data-dump.codeforiati.org/)
- Traverse and parse all activity files and convert them into a JSON
- Improved error handling for malformed XML


---
### Directions
- unset CONDA_PREFIX if both are active
- maturin develop

