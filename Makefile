install:
	cargo build --release
	cp target/release/rush /usr/local/bin/
