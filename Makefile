run:
	@rustc -V
	cargo build
	./target/debug/rush

install:
	cargo build --release
	cp target/release/rush /usr/local/bin/

doc:
	cargo doc --open

clean:
	cargo clean
	find . -name '*.rs.bk' | xargs rm
