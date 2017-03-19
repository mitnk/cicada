install:
	cargo build --release
	cp target/release/rush /usr/local/bin/

doc:
	cargo doc --open


clean:
	find . -name '*.rs.bk' | xargs rm
