run:
	@rustc -V
	cargo build
	./target/debug/cicada

install:
	cargo build --release
	cp target/release/cicada /usr/local/bin/

doc:
	cargo doc --open

test:
	@rustc -V
	cargo test

clean:
	cargo clean
	find . -name '*.rs.bk' | xargs rm -f
