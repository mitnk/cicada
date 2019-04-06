run:
	@rustc -V
	# cargo update
	cargo build
	./target/debug/cicada

install:
	# cargo update
	cargo build --release
	cp target/release/cicada /usr/local/bin/

doc:
	cargo doc --open

test:
	@rustc -V
	cargo test --bins

clippy:
	cargo clippy -- -A clippy::needless_return -A clippy::ptr_arg

clean:
	cargo clean
	find . -name '*.rs.bk' | xargs rm -f
