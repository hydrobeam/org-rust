# how data is collected for the phf_map emojis

import requests


ret_string = ""
data = requests.get(
    "https://cdn.jsdelivr.net/npm/emojibase-data@7.0.1/en/shortcodes/github.json"
).json()


for unicode, shortcodes in data.items():
    if "-" in unicode:
        unicode = unicode.split("-")[0]
    if isinstance(shortcodes, list):
        for item in shortcodes:
            ret_string += f"\"{item}\" => '{chr(int(unicode, 16))}',\n"
    else:
        ret_string += f"\"{shortcodes}\" => '{chr(int(unicode, 16))}',\n"

print(ret_string)

# requests.get("")

# soup = BeautifulSoup(html_doc, 'html.parser')
