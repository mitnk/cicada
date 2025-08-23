# Install to current root by default
DESTDIR := ""
# Install to homedir by default to avoid using root
PREFIX := ${HOME}/.local
# Debug is the default
MODE := debug

ifeq ($(MODE),debug)
	MODEFLAG="--"
else
	MODEFLAG="--$(MODE)"
endif

build:
	cargo build "${MODEFLAG}"

clean:
	cargo clean
	find . -name '*.rs.bk' | xargs -0 rm -f

clippy:
	cargo clippy -- -A clippy::needless_return -A clippy::ptr_arg

doc:
	cargo doc --open

fmt:
	cargo fmt

install: build
	install -Dm755 target/"${MODE}"/cicada "${DESTDIR}"/"${PREFIX}"/bin/cicada

run: build
	cargo run "${MODEFLAG}" -- -l

test: build
	cargo test --bins "${MODEFLAG}"
	MODE="${MODE}" ./tests/test_scripts.sh 2>/dev/null


.PHONY: build clean clippy doc fmt install run test
