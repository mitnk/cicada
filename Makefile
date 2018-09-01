run:
	@rustc -V
	cargo build
	./target/debug/cicada

update:
	cargo update

install:
	cargo build --release
	cp target/release/cicada /usr/local/bin/

doc:
	cargo doc --open

test:
	@rustc -V
	cargo test

clippy:
	cargo clippy -- -A needless_return -A ptr_arg

clean:
	cargo clean
	find . -name '*.rs.bk' | xargs rm -f
