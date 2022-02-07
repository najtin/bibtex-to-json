# bibtex-to-json
This is a simple commandline application to convert a bibtex to json written in Rust and Python.
### Why?
To enable you to convert very big bibtex collections to a more comfortable format within seconds.

The program has two parts:
- 1) A finite state machine to do mark the starting and end positions of the elements
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
#Download a big example BibTeX file
curl -o ir-anthology.bib https://raw.githubusercontent.com/ir-anthology/ir-anthology-data/master/ir-anthology.bib
source .env/bin/activate && cargo run --release ir-anthology.bib ir-anthology.json 
# takes about 10 seconds on a Ryzen 7 5800U 
```
