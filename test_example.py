import iati_json_parser
import json
import time

def test_example_output_string():

    start_time = time.time()
    example_json = iati_json_parser.convert("example/data", pretty=False)

    end_time = time.time()

    print(f"Execution time: {end_time - start_time} seconds")

    with open('example/output.json', 'w') as outfile:
        json.dump(example_json, outfile, indent=4)

test_example_output_string()
