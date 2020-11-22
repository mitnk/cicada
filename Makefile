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

scripting-test:
	cargo build
	./tests/test_scripts.sh 2>/dev/null

clippy:
	cargo clippy -- -A clippy::needless_return -A clippy::ptr_arg

clean:
	cargo clean
	find . -name '*.rs.bk' | xargs rm -f
