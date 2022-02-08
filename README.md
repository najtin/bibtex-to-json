# bibtex-to-json
This is a simple command line application to convert bibtex to json written in Rust and Python.
### Why?
To enable you to convert very big bibtex collections into a more comfortable format within seconds.

The program has two parts:
- 1) A finite state machine to mark the start and end positions of the elements
- 2) Parsing elements with the help of the marked position

The second part is done in parallel and uses all available threads. If a field in a bibtex entry contains latex code, then the content of the field is passed to [pylatexenc](https://github.com/phfaist/pylatexenc) (a latex interpreter for python). Thanks to  Philippe Faist for this awesome python library. Each thread has its own instance of the python interpreter so that they do not block each other through the gil.
### Prerequirements:
- python3 and pip3
- make
- [Cargo](https://www.rust-lang.org/tools/install) 

### How to compile:
```bash
make
```
### How to run with example:
```bash
#run a small example
source .env/bin/activate && cargo run --release examples/single.bib single.json 

#Download a big example BibTeX file
curl -o ir-anthology.bib https://raw.githubusercontent.com/ir-anthology/ir-anthology-data/master/ir-anthology.bib
source .env/bin/activate && cargo run --release ir-anthology.bib ir-anthology.json 
# takes about 10 seconds on a Ryzen 7 5800U 
```

### Output structure
The json is an array of dictionaries. Here is an example structure:
```json
[
  {
    "bibkey": "this string contains the bibkey of the bibtex entry e.g. conf/ecit/Meier2000",
    "entry_type": "this string contains the typ eof the bibtex entry e.g. article or proceedings",
    "original": "the original string of this entry from the parsed bibtex file",
    "fields" : {
      "author" : "the fields dictionary contains all fields, e.g. the author field, a value here could be Max Mustermann",
      "editor" : "here are some more example fields",
      "year" : "2022",
      "pages" : "1--23"
    }
  }
]
```