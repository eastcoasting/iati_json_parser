from .iati_json_parser import convert as convert_rs

def convert(input, pretty=False, schemas=None):
    return convert_rs(input, pretty)
