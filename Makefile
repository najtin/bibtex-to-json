build: python-dependencies
	cargo build --release
python-dependencies:
	python3 -m venv .env && source .env/bin/activate && pip3 install pylatexenc