run:
	@rustc -V
	cargo build
	./target/debug/mtsh

install:
	cargo build --release
	cp target/release/mtsh /usr/local/bin/

doc:
	cargo doc --open

clean:
	cargo clean
	find . -name '*.rs.bk' | xargs rm
